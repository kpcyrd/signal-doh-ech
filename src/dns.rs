use crate::args::Resolve;
use crate::errors::*;
use doh_dns::{client::HyperDnsClient, Dns, DnsHttpsServer};
use std::net::IpAddr;
use std::time::Duration;

pub async fn resolve(name: &str) -> Result<Vec<IpAddr>> {
    // TODO: this shouldn't be hardcoded
    let dns: Dns<HyperDnsClient> = Dns::with_servers(&[
        DnsHttpsServer::Google(Duration::from_secs(2)),
        DnsHttpsServer::Cloudflare1_1_1_1(Duration::from_secs(10)),
    ])?;

    info!("Resolving {:?}", name);
    // TODO: this should resolve ipv4+ipv6 at the same time
    let responses = dns.resolve_a(name).await?;
    if responses.is_empty() {
        bail!("No entries found.")
    }

    let addrs = responses
        .iter()
        .flat_map(|res| {
            debug!(
                "Got dns record: {:?} (type={})",
                res,
                dns.rtype_to_name(res.r#type)
            );
            match res.r#type {
                1 | 28 => res.data.parse().ok(),
                _ => None,
            }
        })
        .collect();

    Ok(addrs)
}

pub async fn run(args: Resolve) -> Result<()> {
    let addrs = resolve(&args.name).await?;
    for addr in addrs {
        println!("{}", addr);
    }
    Ok(())
}
