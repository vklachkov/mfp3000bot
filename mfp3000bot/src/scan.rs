use crate::config::Config;
use anyhow::{anyhow, bail, Context};
use bstr::ByteSlice;
use lazy_static::lazy_static;
use simple_sane::{Backend, FrameFormat, OptionValue, Parameters, Scanner};
use std::{io::Read, thread};
use tokio::sync::{mpsc, oneshot};

lazy_static! {
    static ref BACKEND: Backend = Backend::new().expect("SANE should be initialize successfully");
}

pub enum ScanState {
    Prepair,
    Progress(f64),
    Done(Jpeg),
    Error(anyhow::Error),
    Cancelled,
}

pub struct Jpeg {
    pub bytes: Vec<u8>,
    pub format: JpegFormat,
    pub width: usize,
    pub height: usize,
}

pub enum JpegFormat {
    RGB,
    Gray,
}

pub fn start(config: Config, mut cancel: oneshot::Receiver<()>) -> mpsc::Receiver<ScanState> {
    let (mut state_tx, state_rx) = mpsc::channel(4);

    thread::Builder::new()
        .name("scan".to_owned())
        .spawn(move || {
            match scan_page(config, &mut state_tx, &mut cancel) {
                Ok(Some((parameters, bytes))) => {
                    match raw_image(parameters, bytes) {
                        Ok(raw_image) => {
                            let jpeg = encode_jpeg(raw_image, 70);
                            _ = state_tx.blocking_send(ScanState::Done(jpeg))
                        }
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
                log::debug!("State sender was dropped");
                return Ok(None);
            }
        };
    }
    macro_rules! check_cancellation {
        ($channel:expr) => {
            match $channel.try_recv() {
                Ok(()) => {
                    log::debug!("Scan cancelled");
                    return Ok(None);
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    log::debug!("Cancel sender was dropped");
                    return Ok(None);
                }
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

    // TODO: Don't use get_devices, try to open by name from config.
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

    send_state!(ScanState::Progress(0.0));

    let mut previous_progress = 0.0;
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

        log::trace!(
            "Scan progress {page_offset} of {page_size} bytes ({}%)",
            (page_offset as f64 / page_size as f64 * 100.)
        );

        let progress = page_offset as f64 / page_size as f64 * 100.;
        if progress - previous_progress >= 5.0 {
            send_state!(ScanState::Progress(progress));
            previous_progress = progress;
        }

        page_offset += read;
    }

    send_state!(ScanState::Progress(100.0));

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

fn raw_image(parameters: Parameters, pixels: Vec<u8>) -> anyhow::Result<libjpeg::RawImage> {
    let width = parameters.pixels_per_line;
    let height = parameters.lines;

    let format = match parameters.format {
        FrameFormat::Gray => libjpeg::RawImageFormat::Gray,
        FrameFormat::RGB => libjpeg::RawImageFormat::Rgb,
        format => bail!("unsupported image format '{format:?}'"),
    };

    Ok(libjpeg::RawImage {
        pixels,
        width,
        height,
        format,
    })
}

fn encode_jpeg(image: libjpeg::RawImage, output_quality: u8) -> Jpeg {
    log::trace!("Start jpeg encoding");

    let bytes = unsafe { libjpeg::compress_to_jpeg(&image, output_quality) };

    log::trace!("End jpeg encoding");

    Jpeg {
        bytes,
        format: match image.format {
            libjpeg::RawImageFormat::Rgb => JpegFormat::RGB,
            libjpeg::RawImageFormat::Gray => JpegFormat::Gray,
        },
        width: image.width,
        height: image.height,
    }
}
