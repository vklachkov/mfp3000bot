/*

use crate::config::Config;
use anyhow::{anyhow, bail, Context};
use bstr::ByteSlice;
use image::DynamicImage;
use lazy_static::lazy_static;
use simple_sane::{
    Backend, Device, FrameFormat, OptionCapatibilities, OptionValue, Parameters, Scanner,
};
use std::{
    io::{Cursor, Read},
    thread,
};
use tokio::sync::{mpsc, oneshot};

lazy_static! {
    static ref BACKEND: Backend = Backend::new().expect("SANE should be initialize successfully");
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

pub enum ScanState {
    Prepair,
    Progress(u8),
    Done(::image::DynamicImage),
    Error(anyhow::Error),
    Cancelled,
}

pub fn scan(config: Config, mut cancel: oneshot::Receiver<()>) -> mpsc::Receiver<ScanState> {
    let (mut state_tx, state_rx) = mpsc::channel(4);

    thread::Builder::new()
        .name("scan".to_owned())
        .spawn(move || {
            match scan_page(config, &mut state_tx, &mut cancel) {
                Ok(Some((parameters, bytes))) => {
                    match raw_to_dynamic_image(parameters, bytes) {
                        Ok(image) => _ = state_tx.blocking_send(ScanState::Done(image)),
                        Err(err) => _ = state_tx.blocking_send(ScanState::Error(err)),
                    };
                }
                Ok(None) => {
                    _ = state_tx.blocking_send(ScanState::Cancelled);
                }
                Err(err) => {
                    _ = state_tx.blocking_send(ScanState::Error(err));
                }
            };
        })
        .expect("thread name should be valid");

    state_rx
}

fn scan_page(
    config: Config,
    state: &mut mpsc::Sender<ScanState>,
    cancel: &mut oneshot::Receiver<()>,
) -> anyhow::Result<Option<(Parameters, Vec<u8>)>> {
    macro_rules! send_state {
        ($state:expr) => {
            if state.blocking_send($state).is_err() {
                return Ok(None);
            }
        };
    }
    macro_rules! check_cancellation {
        ($channel:expr) => {
            match $channel.try_recv() {
                Ok(()) => return Ok(None),
                Err(oneshot::error::TryRecvError::Closed) => return Ok(None),
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
        };
    }

    send_state!(ScanState::Prepair);

    let device_name = config
        .devices
        .scanner
        .clone()
        .ok_or_else(|| anyhow!("scanner is not specified in the config"))?;

    log::debug!("Use scanner '{device_name}'");

    check_cancellation!(cancel);
    let device = BACKEND
        .find_device_by_name(&device_name)
        .context("reading devices")?
        .ok_or_else(|| anyhow!("device '{device_name}' not found"))?;

    check_cancellation!(cancel);
    let mut scanner = Scanner::new(device).context("opening device")?;

    setup_scanner(&mut scanner, &config);

    check_cancellation!(cancel);
    let mut reader = scanner.start().context("starting scan")?;

    check_cancellation!(cancel);
    let parameters = reader.get_parameters().context("getting parameters")?;

    log::debug!("Start scan with parameters {parameters:?}");

    let page_size = parameters.bytes_per_line * parameters.lines;
    let mut page = vec![0u8; page_size];
    let mut page_offset = 0;

    send_state!(ScanState::Progress(0));

    let mut previous_progress = 0;
    loop {
        const WINDOW_SIZE: usize = 128 * 1024;

        check_cancellation!(cancel);

        let buf = if page_offset + WINDOW_SIZE < page_size {
            &mut page[page_offset..page_offset + WINDOW_SIZE]
        } else {
            &mut page[page_offset..page_size]
        };

        let read = reader.read(buf).context("reading from scanner")?;
        if read == 0 {
            break;
        }

        let progress = (page_offset as f64 / page_size as f64 * 100.).round() as u8;
        if progress - previous_progress >= 5 {
            send_state!(ScanState::Progress(progress));
            previous_progress = progress;
        }

        page_offset += read;
    }

    send_state!(ScanState::Progress(100));

    check_cancellation!(cancel);

    log::debug!("Scan done");

    Ok(Some((parameters, page)))
}

fn setup_scanner(scanner: &mut Scanner<'_>, config: &Config) {
    let device_name = scanner.get_device().name.to_string();

    let options = scanner.options();
    log::debug!("Start device setup. Available {options:#?}");

    'setup: for (i, option) in options.into_iter().enumerate() {
        let Some(option_name) = option.name else {
            log::debug!("Skip unnamed option #{i}");
            continue;
        };

        'custom_value: {
            let Some(config) = config.scanner.get(&device_name) else {
                log::debug!("No custom options for device '{device_name}'. Use default values");
                break 'custom_value;
            };

            let Some(value) = config.get(option_name) else {
                log::debug!("No custom value for option '{option_name}' (#{i}). Use default value");
                break 'custom_value;
            };

            if let Err(err) = option.set_value(OptionValue::String(value.as_bstr())) {
                log::warn!(
                    "Failed to set '{value}' value for option '{option_name}' (#{i}): {err}"
                );
                break 'custom_value;
            } else {
                log::debug!("Successfully set value '{value}' for option '{option_name}' (#{i})");
                continue 'setup;
            }
        }

        if option.is_auto_settable() {
            if let Err(err) = option.set_auto() {
                log::warn!("Failed to auto configure option '{option_name}' (#{i}): {err}");
            } else {
                log::debug!("Successfully set automatic value for option '{option_name}' (#{i})");
            }
        } else {
            log::debug!("Option '{option_name}' (#{i}) does not support auto value, skip");
        }
    }
}

fn raw_to_dynamic_image(parameters: Parameters, raw: Vec<u8>) -> anyhow::Result<DynamicImage> {
    use image::{GrayImage, RgbImage};

    validate_parameters(parameters)?;

    let width = parameters.pixels_per_line as u32;
    let height = parameters.lines as u32;

    let image = match parameters.format {
        FrameFormat::Gray => DynamicImage::from(
            GrayImage::from_raw(width, height, raw)
                .context("creating image from scanner buffer")?,
        ),
        FrameFormat::RGB => DynamicImage::from(
            RgbImage::from_raw(width, height, raw).context("creating image from scanner buffer")?,
        ),
        format => bail!("unsupported image format '{format:?}'"),
    };

    Ok(image)
}

fn validate_parameters(parameters: Parameters) -> anyhow::Result<()> {
    if parameters.pixels_per_line > u32::MAX as usize {
        bail!(
            "parameters.pixels_per_line ({}) is bigger than u32::MAX ({})",
            parameters.pixels_per_line,
            u32::MAX
        );
    }

    if parameters.lines > u32::MAX as usize {
        bail!(
            "parameters.lines ({}) is bigger than u32::MAX ({})",
            parameters.pixels_per_line,
            u32::MAX
        );
    }

    Ok(())
}

*/
