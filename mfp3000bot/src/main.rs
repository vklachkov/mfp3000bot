mod bot;
mod bot_data;
mod config;
// mod pdf_builder;
mod print;
mod scan;

use argh::FromArgs;
use config::Config;
use log::Level;
use std::{path::PathBuf, process};

#[derive(FromArgs)]
/// Telegram bot for printing and scanning
struct Args {
    /// path to config
    #[argh(option)]
    config: PathBuf,

    /// enable extra logs
    #[argh(switch)]
    verbose: bool,

    /// enable trace logs
    #[argh(switch)]
    trace: bool,
}

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();

    setup_logger(&args);
    hello(&args);

    let config = read_config(&args);

    log::info!("Start telegram bot");
    bot::start(config).await;
}

fn setup_logger(args: &Args) {
    simple_logger::init_with_level(if args.trace {
        Level::Trace
    } else if args.verbose {
        Level::Debug
    } else {
        Level::Info
    })
    .unwrap();
}

fn hello(args: &Args) {
    log::info!(
        "{bin} version {version}, commit {commit}, config from {config_path}, verbose {verbose}",
        bin = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        commit = env!("GIT_COMMIT_HASH"),
        config_path = args.config.display(),
        verbose = if args.verbose { "on" } else { "off" },
    );
}

fn read_config(args: &Args) -> Config {
    match Config::read_from(&args.config) {
        Ok(config) => {
            log::debug!("Use config {config:#?}");
            config
        }
        Err(err) => {
            log::error!("Failed to read config: {err:#}");
            process::exit(1);
        }
    }
}
