mod config;
mod pdf_builder;
mod print;
mod scan;

use std::{
    io::{BufWriter, Cursor},
    path::PathBuf,
    process, thread,
    time::Duration,
};

use argh::FromArgs;
use config::Config;
use image::DynamicImage;
use log::Level;
use pdf_builder::PdfBuilder;
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

    /// enable trace logs
    #[argh(switch)]
    trace: bool,
}

#[tokio::main]
async fn main() {
    let args: Args = argh::from_env();

    simple_logger::init_with_level(if args.trace {
        Level::Trace
    } else if args.verbose {
        Level::Debug
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
                test_scan_to_pdf(bot, msg, config).await;
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

async fn test_scan_to_pdf(
    bot: Bot,
    msg: Message,
    config: Config,
) -> teloxide::requests::ResponseResult<()> {
    bot.send_message(msg.chat.id, "Изображение 1 в процессе...")
        .await?;

    let img1 = get_image_from_scanner(config.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(250));

    bot.send_message(msg.chat.id, "Изображение 2 в процессе...")
        .await?;

    let img2 = get_image_from_scanner(config.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(250));

    bot.send_message(msg.chat.id, "Изображение 3 в процессе...")
        .await?;

    let img3 = get_image_from_scanner(config.clone()).await.unwrap();

    bot.send_message(msg.chat.id, "Собираю PDF").await?;

    let pdf = imgs_to_pdf(vec![img1, img2, img3]).await;

    bot.send_document(msg.chat.id, InputFile::memory(pdf).file_name("Scanned.pdf"))
        .await?;

    Ok(())
}

async fn imgs_to_pdf(imgs: Vec<DynamicImage>) -> Vec<u8> {
    let (tx, rx) = oneshot::channel::<Vec<u8>>();

    thread::spawn(move || {
        let pdf_builder = PdfBuilder::new("Document", 300.0);

        for img in imgs {
            pdf_builder.add_image(img).unwrap();
        }

        let mut pdf = Vec::new();
        pdf_builder.write_to(&mut pdf).unwrap();

        tx.send(pdf).unwrap();
    });

    rx.await.unwrap()
}

async fn get_image_from_scanner(config: Config) -> Option<DynamicImage> {
    let (cancel_tx, cancel_rx) = oneshot::channel();
    let mut scan_state = scan(config, cancel_rx);

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
            ScanState::Done(img) => {
                // bot.send_photo(msg.chat.id, InputFile::memory(jpeg)).await?;
                // bot.edit_message_text(msg.chat.id, msg.id, "Готово").await?;
                return Some(img);
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

    None
}
