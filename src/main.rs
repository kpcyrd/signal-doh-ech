use env_logger::Env;
use signal_doh_ech::args::{Args, SubCommand};
use signal_doh_ech::backend;
use signal_doh_ech::connect;
use signal_doh_ech::dns;
use signal_doh_ech::errors::*;
use signal_doh_ech::tunnel;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::from_args();
    let level = match args.verbose {
        0 => "off",
        1 => "info",
        2 => "info,signal_doh_ech=debug",
        3 => "debug",
        _ => "debug,signal_doh_ech=trace",
    };
    env_logger::init_from_env(Env::default().default_filter_or(level));

    match args.subcommand {
        SubCommand::Connect(args) => connect::run(args).await?,
        SubCommand::Resolve(args) => dns::run(args).await?,
        SubCommand::Tunnel(args) => tunnel::run(args).await?,
        SubCommand::Backend(args) => backend::run(args).await?,
        // SubCommand::Ping(_args) => (),
        SubCommand::Completions(args) => args.gen_completions()?,
    }

    Ok(())
}
