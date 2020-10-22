use crate::errors::*;
use nom::bytes::complete::tag;
use nom::bytes::complete::take;
use nom::number::complete::{be_u16, be_u8};
use nom::IResult;
use std::convert::TryFrom;
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct Request {
    addr: Dest,
    pub port: u16,
}

impl Request {
    pub fn to_host_addr(&self) -> Result<String> {
        match &self.addr {
            Dest::IPv4(addr) => Ok(addr.to_string()),
            Dest::Domain(addr) => Ok(addr.to_string()),
            Dest::IPv6(addr) => Ok(addr.to_string()),
            Dest::Invalid => bail!("Invalid socks5 request"),
        }
    }

    pub fn to_sock_addr(&self) -> Result<String> {
        // TODO: not very elegant, maybe change the protocol to accept Request
        match &self.addr {
            Dest::IPv4(addr) => Ok(format!("{}:{}", addr, self.port)),
            Dest::Domain(addr) => Ok(format!("{}:{}", addr, self.port)),
            Dest::IPv6(addr) => Ok(format!("[{}]:{}", addr, self.port)),
            Dest::Invalid => bail!("Invalid socks5 request"),
        }
    }
}

#[derive(Debug)]
enum Dest {
    IPv4(Ipv4Addr),
    Domain(String),
    IPv6(Ipv6Addr),
    Invalid,
}

enum State {
    PreAuth,
    PostAuth,
}

pub async fn handshake(sock: &mut TcpStream) -> Result<Request> {
    let mut i = 0;
    let mut buf = [0u8; 4096];

    let mut state = State::PreAuth;

    loop {
        let n = sock.read(&mut buf[i..]).await?;
        if n == 0 {
            bail!("Client disconnected");
        }
        i += n;
        trace!("Received data {:?}", &buf[..i]);

        match state {
            State::PreAuth => {
                if let Ok((bytes, _)) = parse_handshake_a(&buf[..i]) {
                    trace!("Moving into post auth, remaining in buffer: {:?}", bytes);
                    // pick unauthenticated
                    sock.write_all(b"\x05\x00").await?;
                    // move into next state of handshake
                    state = State::PostAuth;
                    // discard everything we got so far
                    i = 0;
                }
            }
            State::PostAuth => {
                if let Ok((bytes, req)) = parse_handshake_b(&buf[..i]) {
                    if !bytes.is_empty() {
                        bail!("Found trailing data after socks5 handshake: {:?}", bytes);
                    }
                    info!("Received socks5 request: {:?}", req.to_sock_addr()?);
                    sock.write_all(b"\x05\x00\x00\x01\x00\x00\x00\x00\x00\x00")
                        .await?;
                    return Ok(req);
                }
            }
        }

        if i == buf.len() {
            bail!("Giving up during socks5 handshake, buffer full");
        }
    }
}

fn parse_handshake_a(bytes: &[u8]) -> IResult<&[u8], ()> {
    let (bytes, _) = tag(b"\x05")(bytes)?;
    // read supported auths and discard
    let (bytes, len) = be_u8(bytes)?;
    let (bytes, _) = take(len)(bytes)?;
    Ok((bytes, ()))
}

fn parse_handshake_b(bytes: &[u8]) -> IResult<&[u8], Request> {
    let (bytes, _) = tag(b"\x05\x01\x00")(bytes)?;
    let (bytes, family) = be_u8(bytes)?;

    let (bytes, addr) = match family {
        // ipv4
        0x01 => {
            let (bytes, addr) = nom::bytes::complete::take(4u8)(bytes)?;
            let addr = Ipv4Addr::from(<[u8; 4]>::try_from(addr).unwrap());
            (bytes, Dest::IPv4(addr))
        }
        // domain
        0x03 => {
            let (bytes, len) = be_u8(bytes)?;
            let (bytes, domain) = take(len)(bytes)?;
            if let Ok(s) = String::from_utf8(domain.to_vec()) {
                (bytes, Dest::Domain(s))
            } else {
                (bytes, Dest::Invalid)
            }
        }
        // ipv6
        0x04 => {
            let (bytes, addr) = nom::bytes::complete::take(16u8)(bytes)?;
            let addr = Ipv6Addr::from(<[u8; 16]>::try_from(addr).unwrap());
            (bytes, Dest::IPv6(addr))
        }
        _ => (bytes, Dest::Invalid),
    };

    let (bytes, port) = be_u16(bytes)?;
    Ok((bytes, (Request { addr, port })))
}
