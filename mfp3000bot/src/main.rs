mod config;
mod print;
mod scan;

use std::{path::PathBuf, process};

use argh::FromArgs;
use config::Config;
use log::Level;
use print::print_remote_file;
use scan::{scan, ScanState};
use teloxide::{prelude::*, types::InputFile};
use tokio::sync::oneshot;

#[derive(FromArgs)]
/// Telegram bot for printing and scanning
struct Args {
    /// path to config
    #[argh(option)]
    config: PathBuf,

    /// enable extra logs
    #[argh(switch)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();

    simple_logger::init_with_level(if args.verbose {
        Level::Trace
    } else {
        Level::Info
    })
    .unwrap();

    hello(&args);

    let config = match Config::read_from(args.config) {
        Ok(config) => config,
        Err(err) => {
            log::error!("Failed to read config: {err:#}");
            process::exit(1);
        }
    };

    if args.verbose {
        log::debug!("Use config {config:#?}");
    }

    log::info!("Start telegram bot");
    telegram_bot(config, args.verbose).await;
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

async fn telegram_bot(config: Config, verbose: bool) {
    let bot = Bot::new(&config.telegram.token);

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let config = config.clone();
        async move {
            log::debug!("Message: {msg:?}");

            let Some(doc) = msg.document() else {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                let mut scan_state = scan(config, cancel_rx);

                let msg = bot
                    .send_message(msg.chat.id, "Ожидание ответа от сервера...")
                    .await?;

                while let Some(state) = scan_state.recv().await {
                    match state {
                        ScanState::Prepair => {
                            // bot.edit_message_text(msg.chat.id, msg.id, "Подготовка к сканированию")
                            //     .await?;
                        }
                        ScanState::Progress(p) => {
                            // bot.edit_message_text(msg.chat.id, msg.id, format!("Прогресс {p}%"))
                            //     .await?;
                        }
                        ScanState::Done(jpeg) => {
                            bot.send_photo(msg.chat.id, InputFile::memory(jpeg)).await?;
                            // bot.edit_message_text(msg.chat.id, msg.id, "Готово").await?;
                        }
                        ScanState::Error(err) => {
                            // bot.edit_message_text(
                            //     msg.chat.id,
                            //     msg.id,
                            //     format!("Ошибка сканирования: {err:#}"),
                            // )
                            // .await?;
                        }
                        ScanState::Cancelled => {
                            // bot.edit_message_text(
                            //     msg.chat.id,
                            //     msg.id,
                            //     format!("Сканирование отменено"),
                            // )
                            // .await?;
                        }
                    };
                }

                return Ok(());
            };

            let file = bot.get_file(&doc.file.id).send().await?;
            let file_name = doc.file_name.as_deref().unwrap_or("(telegram file)");

            let token = config.telegram.token;
            let file_path = file.path;
            let file_url = bot
                .api_url()
                .join(&format!("file/bot{token}/{file_path}"))
                .expect("url should be valid");

            let (tx, rx) = oneshot::channel();

            print_remote_file(file_name.to_owned(), file_url.clone(), tx);

            let result = rx.await.expect("recv print result should be successful");
            match result {
                Ok(()) => {
                    bot.send_message(msg.chat.id, "Файл распечатан!").await?;
                }
                Err(err) => {
                    log::error!("Failed to print file '{file_name}': {err}");
                    bot.send_message(msg.chat.id, "Ошибка печати").await?;
                }
            }

            Ok(())
        }
    })
    .await;
}
