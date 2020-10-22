use crate::errors::*;
use std::io::stdout;
use structopt::clap::{AppSettings, Shell};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
pub struct Args {
    /// Verbose logging output (Can be set multiple times)
    #[structopt(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,
    #[structopt(long, global = true, default_value = "1.1.1.1", env = "SDE_DOH_IP")]
    pub resolver_ip: String,
    #[structopt(long, global = true, default_value = "1.1.1.1", env = "SDE_DOH_NAME")]
    pub resolver_name: String,
    #[structopt(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Debug, Clone, StructOpt)]
pub enum SubCommand {
    Connect(Connect),
    Resolve(Resolve),
    Tunnel(Tunnel),
    Backend(Backend),
    // Ping(Ping),
    Completions(Completions),
}

#[derive(Debug, Clone, StructOpt)]
pub struct Proxy {
    /// The websocket proxy server to connect to with TLSv1.3+ECH. Format is example.com, port 443 and https is implied.
    #[structopt(long = "proxy", env = "SDE_PROXY")]
    pub proxy_addr: String,
    #[structopt(long = "proxy-port", default_value = "443")]
    pub proxy_port: u16,
    /// Use ws:// instead of wss://
    #[structopt(long)]
    pub skip_tls: bool,
}

/// Setup a tunnel with TLSv1.3+ECH and connect to a specific address
#[derive(Debug, Clone, StructOpt)]
pub struct Connect {
    #[structopt(flatten)]
    pub proxy: Proxy,
    /// The address to connect to through the proxy
    pub addr: String,
}

/// Resolve a specific name with dns-over-https
#[derive(Debug, Clone, StructOpt)]
pub struct Resolve {
    pub name: String,
}

/// Run a local socks5 server that forwards signal traffic through TLSv1.3+ECH
#[derive(Debug, Clone, StructOpt)]
pub struct Tunnel {
    #[structopt(flatten)]
    pub proxy: Proxy,
    #[structopt(short = "F", long)]
    pub forward: Vec<String>,
    #[structopt(long, default_value = "127.0.0.1:1090")]
    pub bind: String,
}

/// Run the backend proxy server
#[derive(Debug, Clone, StructOpt)]
pub struct Backend {
    #[structopt(short = "A", long = "allow")]
    pub allowed: Vec<String>,
    /// Ping interval in seconds to prevent connection timeouts
    #[structopt(long)]
    pub ping_interval: Option<u64>,
}

/*
/// Check if we can successfully tunnel to signal servers
#[derive(Debug, Clone, StructOpt)]
pub struct Ping {
    /// The websocket proxy server to connect to with TLSv1.3+ECH
    #[structopt(long, env = "SDE_PROXY_URL")]
    pub proxy: String,
}
*/

/// Generate shell completions
#[derive(Debug, Clone, StructOpt)]
pub struct Completions {
    #[structopt(possible_values=&Shell::variants())]
    pub shell: Shell,
}

impl Completions {
    pub fn gen_completions(&self) -> Result<()> {
        Args::clap().gen_completions_to("signal-doh-ech", self.shell, &mut stdout());
        Ok(())
    }
}
