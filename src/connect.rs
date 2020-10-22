use crate::args::{Connect, Proxy};
use crate::common::{Hello, HelloResponse};
use crate::dns;
use crate::errors::*;
use async_tungstenite::tokio::TokioAdapter;
use async_tungstenite::tungstenite::Message;
use async_tungstenite::WebSocketStream;
use futures::task::{self, Poll};
use futures::{select, FutureExt, SinkExt, StreamExt};
use http::Request;
use rustls::ClientConfig;
use std::io;
use std::marker::Unpin;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use webpki::DNSNameRef;

async fn connect(ips: &[IpAddr], port: u16) -> Result<TcpStream> {
    debug!("Trying all of: {:?}", ips);
    for ip in ips {
        let addr = SocketAddr::new(*ip, port);
        match TcpStream::connect(&addr).await {
            Ok(tcp) => {
                info!("Connected to {}", addr);
                return Ok(tcp);
            }
            Err(err) => error!("Connection failed: {:#}", err),
        }
    }
    bail!("Every attempt failed")
}

pub async fn connect_dns(proxy: &str, port: u16) -> Result<TcpStream> {
    let ips = if let Ok(ip) = proxy.parse::<IpAddr>() {
        vec![ip]
    } else {
        dns::resolve(proxy).await?
    };
    connect(&ips, port).await
}

async fn setup_tls(stream: TcpStream, proxy: &str) -> Result<TlsStream<TcpStream>> {
    info!("Negotiating tls connection ({:?})", proxy);

    let mut config = ClientConfig::new();
    config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let config = TlsConnector::from(Arc::new(config));
    let dnsname = DNSNameRef::try_from_ascii_str(proxy)?;

    let tls = config.connect(dnsname, stream).await?;
    Ok(tls)
}

async fn setup_ws<T: AsyncRead + AsyncWrite + Unpin>(
    stream: T,
    proxy: &str,
) -> Result<WebSocketStream<async_tungstenite::tokio::TokioAdapter<T>>> {
    let url = format!("ws://{}/connect", proxy);
    info!("Establishing websocket with {:?}", url);
    let req = Request::get(url).body(()).unwrap();

    let (sock, _resp) = async_tungstenite::tokio::client_async(req, stream)
        .await
        .context("Failed to establish websocket connection")?;

    Ok(sock)
}

async fn req_proxy<T: AsyncRead + AsyncWrite + Unpin>(
    sock: &mut WebSocketStream<TokioAdapter<T>>,
    dest: &str,
) -> Result<()> {
    let hello = Hello::new(dest);
    debug!("Sending hello: {:?}", hello);
    let hello = serde_json::to_vec(&hello)?;
    sock.send(Message::binary(hello)).await?;

    let msg = sock
        .next()
        .await
        .ok_or_else(|| anyhow!("No hello response received"))?
        .context("Failed to read hello response")?;
    if let Message::Binary(msg) = msg {
        let _msg = HelloResponse::parse(&msg)?;
    } else {
        bail!("Unexpected websocket pkt: {:?}", msg);
    }

    info!("Connected");

    Ok(())
}

pub struct Stdio {
    stdin: tokio::io::Stdin,
    stdout: tokio::io::Stdout,
}

impl Default for Stdio {
    fn default() -> Stdio {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        Stdio { stdin, stdout }
    }
}

impl AsyncRead for Stdio {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stdio {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stdout).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stdout).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stdout).poll_shutdown(cx)
    }
}

async fn relay<A: AsyncRead + AsyncWrite + Unpin, B: AsyncRead + AsyncWrite + Unpin>(
    mut ws: WebSocketStream<TokioAdapter<A>>,
    mut stream: B,
) -> Result<()> {
    let mut buf = [0u8; 4096];

    loop {
        select! {
            n = stream.read(&mut buf).fuse() => {
                let n = n?;
                if n == 0 {
                    debug!("Received eof on stdin, closing");
                    break;
                }
                let msg = &buf[..n];
                trace!("Send: {:?}", msg);
                ws.send(Message::binary(msg)).await?;
            },
            msg = ws.next().fuse() => {
                trace!("Recv: {:?}", msg);
                match msg {
                    Some(Ok(msg)) => {
                        if let Message::Binary(buf) = msg {
                            stream.write_all(&buf).await?;
                        }
                    },
                    Some(Err(err)) => {
                        info!("Received websocket error: {:?}", err);
                        break;
                    },
                    None => {
                        debug!("Received eof from ws, closing");
                        break;
                    }
                }
            },
        };
    }
    debug!("Closing connection");
    ws.close(None).await.ok();

    Ok(())
}

pub async fn run_with<T: AsyncRead + AsyncWrite + Unpin>(
    args: &Proxy,
    addr: &str,
    local: T,
) -> Result<()> {
    let stream = connect_dns(&args.proxy_addr, args.proxy_port).await?;

    let proxy = &args.proxy_addr;
    if args.skip_tls {
        let mut stream = setup_ws(stream, proxy)
            .await
            .context("Failed to setup websocket")?;
        req_proxy(&mut stream, &addr).await?;
        relay(stream, local).await
    } else {
        let stream = setup_tls(stream, proxy)
            .await
            .context("Failed to setup tls connection")?;
        let mut stream = setup_ws(stream, proxy)
            .await
            .context("Failed to setup websocket")?;
        req_proxy(&mut stream, &addr).await?;
        relay(stream, local).await
    }
}

pub async fn run(args: Connect) -> Result<()> {
    run_with(&args.proxy, &args.addr, Stdio::default()).await
}
