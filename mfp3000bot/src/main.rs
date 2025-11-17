#[macro_use]
extern crate rust_i18n;

mod bot;
mod bot_utils;
mod config;
#[cfg(feature = "scan")]
mod pdf_builder;
mod print;
#[cfg(feature = "scan")]
mod scan;
#[cfg(feature = "scan")]
mod scan_bot_buttons;

use argh::FromArgs;
use config::Config;
use rust_i18n::{available_locales, i18n, set_locale};
use std::{path::PathBuf, process};

i18n!();

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

    setup_localization(&config);

    log::info!("Start telegram bot");
    bot::start(config).await;
}

fn setup_logger(args: &Args) {
    let level = if args.trace {
        log::LevelFilter::Trace
    } else if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    simple_logger::SimpleLogger::new()
        .with_module_level("reqwest", log::LevelFilter::Off)
        .with_level(level)
        .init()
        .unwrap()
}

fn hello(args: &Args) {
    log::info!(
        "{bin} version {version}, commit {commit}, config from {config_path}, {verbose}",
        bin = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        commit = env!("GIT_COMMIT_HASH"),
        config_path = args.config.display(),
        verbose = if args.trace {
            "trace enabled"
        } else if args.verbose {
            "verbose enabled"
        } else {
            "additional logs disabled"
        },
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

fn setup_localization(config: &Config) {
    let locale = &config.telegram.language;

    let supported_locales = available_locales!();
    if !supported_locales.contains(&locale.as_str()) {
        log::error!(
            "Unsupported language '{locale}'. Supported languages: {}",
            supported_locales.join(", ")
        );
        process::exit(1);
    }

    set_locale(locale);
}
