use crate::args::Backend;
use crate::common::{Hello, HelloResponse};
use crate::errors::*;
use crate::rules;
use futures::{select, FutureExt, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

async fn handle(args: Arc<Backend>, mut ws: WebSocket) -> Result<()> {
    info!("Websocket client connected");
    let hello = ws
        .next()
        .await
        .ok_or_else(|| anyhow!("No hello msg received"))?
        .context("Failed to read hello msg")?;
    let hello = Hello::parse(hello.as_bytes())?;
    debug!("Received hello pkt: {:?}", hello);

    if !rules::matches(&hello.addr, &args.allowed) {
        bail!("Requested destination is not allowed: {:?}", hello.addr);
    }

    // TODO: timeouts
    info!("Connecting to {:?}", hello.addr);
    let mut remote = TcpStream::connect(&hello.addr)
        .await
        .with_context(|| anyhow!("Failed to connect to destination: {:?}", hello.addr))?;

    info!("Confirming successful connection");
    let msg = HelloResponse::Accepted.to_vec()?;
    ws.send(Message::binary(msg)).await?;

    let mut buf = [0u8; 1024];
    loop {
        select! {
            n = remote.read(&mut buf).fuse() => {
                let n = n?;
                if n == 0 {
                    debug!("Received eof from remote, closing");
                    break;
                }
                let msg = &buf[..n];
                trace!("Recv: {:?}", msg);
                ws.send(Message::binary(msg)).await?;
            }
            msg = ws.next().fuse() => {
                match msg {
                    Some(Ok(msg)) => {
                        if msg.is_binary() {
                            trace!("Send: {:?}", msg);
                            remote.write(msg.as_bytes()).await?;
                        }
                    },
                    Some(Err(err)) => {
                        info!("Received websocket error: {:?}", err);
                        break;
                    },
                    None => {
                        debug!("Received eof from ws, closing");
                        break;
                    },
                }
            }
        }
    }

    debug!("Closing connection");
    ws.close().await.ok();

    Ok(())
}

pub async fn run(args: Backend) -> Result<()> {
    let args = Arc::new(args);
    let routes = warp::path("connect")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let args = Arc::clone(&args);
            ws.on_upgrade(|ws| {
                handle(args, ws).map(|res| {
                    if let Err(e) = res {
                        warn!("Websocket client disconnected: {:?}", e);
                    }
                })
            })
        });

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    Ok(())
}
