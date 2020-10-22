use crate::args::Tunnel;
use crate::connect;
use crate::errors::*;
use crate::rules;
use crate::socks5;
use futures::{select, FutureExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;

async fn process(args: Arc<Tunnel>, mut sock: TcpStream, _addr: SocketAddr) -> Result<()> {
    let req = socks5::handshake(&mut sock).await?;
    let addr = req.to_sock_addr()?;

    if rules::matches(&addr, &args.forward) {
        info!("Forwarding connection to proxy: {:?}", addr);
        connect::run_with(&args.proxy, &addr, sock).await
    } else {
        info!("Creating direct connection");
        let remote = connect::connect_dns(&req.to_host_addr()?, req.port).await?;
        relay(remote, sock).await
    }
}

async fn relay<A: AsyncRead + AsyncWrite + Unpin, B: AsyncRead + AsyncWrite + Unpin>(
    mut remote: A,
    mut local: B,
) -> Result<()> {
    let mut buf_a = [0u8; 4096];
    let mut buf_b = [0u8; 4096];

    loop {
        select! {
            n = remote.read(&mut buf_a).fuse() => {
                let n = n?;
                if n == 0 {
                    debug!("Received eof on stdin, closing");
                    break;
                }
                let msg = &buf_a[..n];
                trace!("Recv: {:?}", msg);
                local.write_all(&msg).await?;
            },
            n = local.read(&mut buf_b).fuse() => {
                let n = n?;
                if n == 0 {
                    debug!("Received eof on stdin, closing");
                    break;
                }
                let msg = &buf_b[..n];
                trace!("Send: {:?}", msg);
                remote.write_all(&msg).await?;
            },
        };
    }
    debug!("Closing connection");

    Ok(())
}

pub async fn run(args: Tunnel) -> Result<()> {
    let mut listener = TcpListener::bind(&args.bind).await?;
    info!("Started socks5 server on {:?}", args.bind);

    let config = Arc::new(args);
    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("Connection from {:?}", addr);
        let config = Arc::clone(&config);
        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            if let Err(e) = process(config, stream, addr).await {
                warn!("An error occurred; error = {:#}", e);
            }
        });
    }
}
