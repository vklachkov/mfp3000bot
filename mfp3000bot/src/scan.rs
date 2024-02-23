use crate::config::Config;
use anyhow::{anyhow, bail, Context};
use bstr::ByteSlice;
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

pub enum ScanState {
    Prepair,
    Progress(u8),
    Done(RawImage),
    Error(anyhow::Error),
    Cancelled,
}

pub struct RawImage {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

pub fn scan(config: Config, mut cancel: oneshot::Receiver<()>) -> mpsc::Receiver<ScanState> {
    let (mut state_tx, state_rx) = mpsc::channel(4);

    thread::Builder::new()
        .name("scan".to_owned())
        .spawn(move || {
            match scan_page(config, &mut state_tx, &mut cancel) {
                Ok(Some((parameters, bytes))) => {
                    _ = state_tx.blocking_send(ScanState::Done(RawImage {
                        bytes,
                        width: parameters.pixels_per_line,
                        height: parameters.lines,
                    }));

                    // match encode_jpeg(parameters, raw, 75) {
                    //     Ok(jpeg) => _ = state_tx.blocking_send(ScanState::Done(jpeg)),
                    //     Err(err) => _ = state_tx.blocking_send(ScanState::Error(err)),
                    // };
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
        .ok_or_else(|| anyhow!("scanner is not specified in the config"))?;

    log::info!("Use scanner '{device_name}'");

    check_cancellation!(cancel);
    let device = BACKEND
        .find_device_by_name(&device_name)
        .context("reading devices")?
        .ok_or_else(|| anyhow!("device '{device_name}' not found"))?;

    check_cancellation!(cancel);
    let mut scanner = Scanner::new(device).context("opening device")?;

    let options = scanner.options();
    log::debug!("Available {options:#?}");

    'l: for option in options {
        let Some(option_name) = option.name else {
            log::debug!("");
            continue;
        };

        'setup: {
            let Some(config) = config.scanner.get(&device_name) else {
                log::trace!("");
                break 'setup;
            };

            let Some(value) = config.get(option_name) else {
                log::trace!("");
                break 'setup;
            };

            if let Err(err) = option.set_value(OptionValue::String(value.as_bstr())) {
                log::warn!("");
                break 'setup;
            } else {
                log::info!("");
                continue 'l;
            }
        }

        if option
            .capatibilities
            .contains(OptionCapatibilities::Automatic)
        {
            if let Err(err) = option.set_auto() {
                log::warn!("");
            } else {
                log::debug!("");
            }
        } else {
            log::debug!("");
        }
    }

    check_cancellation!(cancel);
    let mut reader = scanner.start().context("starting scan")?;

    check_cancellation!(cancel);
    let parameters = reader.get_parameters().context("getting parameters")?;

    log::debug!("Use {parameters:#?}");

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

    Ok(Some((parameters, page)))
}

fn encode_jpeg(
    parameters: Parameters,
    raw: Vec<u8>,
    output_quality: u8,
) -> anyhow::Result<Vec<u8>> {
    use image::{GrayImage, ImageOutputFormat, RgbImage};

    validate_parameters(parameters)?;

    let width = parameters.pixels_per_line as u32;
    let height = parameters.lines as u32;

    let mut image = Vec::new();
    let format = ImageOutputFormat::Jpeg(output_quality);

    match parameters.format {
        FrameFormat::Gray => {
            GrayImage::from_raw(width, height, raw)
                .context("creating image from scanner buffer")?
                .write_to(&mut Cursor::new(&mut image), format)
                .unwrap();
        }
        FrameFormat::RGB => {
            RgbImage::from_raw(width, height, raw)
                .expect("creating image from scanner buffer")
                .write_to(&mut Cursor::new(&mut image), format)
                .unwrap();
        }
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
