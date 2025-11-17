use crate::config::Config;
#[cfg(not(feature = "scan"))]
use crate::config::{Devices, Scan};
use anyhow::anyhow;
use tokio::sync::{mpsc, oneshot};

#[cfg(feature = "scan")]
use anyhow::{bail, Context};
#[cfg(feature = "scan")]
use bstr::{BStr, BString};
#[cfg(feature = "scan")]
use lazy_static::lazy_static;
#[cfg(feature = "scan")]
use libsane::{Backend, FrameFormat, OptionValue, Parameters, Scanner, ScannerOption};
#[cfg(feature = "scan")]
use std::{collections::HashMap, io::Read, thread};

#[cfg(feature = "scan")]
lazy_static! {
    static ref BACKEND: Backend = Backend::new().expect("SANE should be initialize successfully");
}

#[cfg_attr(not(feature = "scan"), allow(dead_code))]
pub enum ScanState {
    Prepair,
    Progress,
    Stop,
    CompressToJpeg,
    Done(Jpeg),
    Error(anyhow::Error),
    Cancelled,
}

#[derive(Clone)]
pub struct Jpeg {
    pub bytes: Vec<u8>,
    pub format: JpegFormat,
    pub width: usize,
    pub height: usize,
}

#[cfg_attr(not(feature = "scan"), allow(dead_code))]
#[derive(Clone, Copy)]
pub enum JpegFormat {
    Rgb,
    Gray,
}

#[cfg(feature = "scan")]
pub fn start(
    config: Config,
    dpi: u16,
    mut cancel: oneshot::Receiver<()>,
) -> mpsc::Receiver<ScanState> {
    let (mut state_tx, state_rx) = mpsc::channel(4);

    thread::Builder::new()
        .name("scan".to_owned())
        .spawn(move || {
            let scan_result = scan_page(config, dpi, &mut state_tx, &mut cancel);
            match scan_result {
                Ok(true) => {}
                Ok(false) => {
                    _ = state_tx.blocking_send(ScanState::Cancelled);
                }
                Err(err) => {
                    _ = state_tx.blocking_send(ScanState::Error(err));
                }
            }
        })
        .expect("thread name should be valid");

    state_rx
}

#[cfg(not(feature = "scan"))]
pub fn start(
    config: Config,
    _dpi: u16,
    _cancel: oneshot::Receiver<()>,
) -> mpsc::Receiver<ScanState> {
    let Config {
        devices: Devices {
            scanner: device_scanner,
            ..
        },
        scanner: scanner_overrides,
        scan: Scan {
            page_quality,
            common_options,
            ..
        },
        ..
    } = config;
    let _ = (
        device_scanner,
        scanner_overrides,
        page_quality,
        common_options,
    );

    let (state_tx, state_rx) = mpsc::channel(1);

    log::warn!("Scan feature was disabled at build time");

    let _ = state_tx.blocking_send(ScanState::Error(anyhow!(
        "scan feature is disabled at build time"
    )));

    state_rx
}

#[cfg(feature = "scan")]
fn scan_page(
    config: Config,
    dpi: u16,
    state: &mut mpsc::Sender<ScanState>,
    cancel: &mut oneshot::Receiver<()>,
) -> anyhow::Result<bool> {
    macro_rules! send_state {
        ($state:expr) => {
            if state.blocking_send($state).is_err() {
                log::debug!("State sender was dropped");
                return Ok(false);
            }
        };
    }
    macro_rules! check_cancellation {
        ($channel:expr) => {
            match $channel.try_recv() {
                Ok(()) => {
                    log::debug!("Scan cancelled");
                    return Ok(false);
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    log::debug!("Cancel sender was dropped");
                    return Ok(false);
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
        };
    }

    send_state!(ScanState::Prepair);

    let scanner_name = config
        .devices
        .scanner
        .clone()
        .ok_or_else(|| anyhow!("scanner is not specified in the config"))?;

    check_cancellation!(cancel);
    let mut scanner = Scanner::new(&scanner_name).context("opening device")?;

    setup_scanner(&mut scanner, &scanner_name, &config, dpi);

    check_cancellation!(cancel);
    let mut reader = scanner.start().context("starting scan")?;

    check_cancellation!(cancel);
    let parameters = reader.get_parameters().context("getting parameters")?;

    log::debug!("Start scan with parameters {parameters:?}");

    check_cancellation!(cancel);
    send_state!(ScanState::Progress);

    let mut pixels = vec![0u8; parameters.bytes_per_line * parameters.lines];
    let mut pixels_offset = 0;

    let mut scanline = vec![0u8; parameters.bytes_per_line];
    let mut scanline_offset = 0;

    loop {
        check_cancellation!(cancel);

        let buf = &mut scanline[scanline_offset..];
        let read = reader.read(buf).context("reading from scanner")?;
        if read == 0 {
            break;
        }

        if pixels_offset == pixels.len() {
            bail!("sane_read() returns {read} bytes, but page has already been read");
        }

        scanline_offset += read;

        if scanline_offset == scanline.len() {
            pixels[pixels_offset..pixels_offset + scanline.len()].copy_from_slice(&scanline);
            pixels_offset += scanline.len();

            scanline_offset = 0;
        }
    }

    check_cancellation!(cancel);
    send_state!(ScanState::Stop);

    drop(reader);
    drop(scanner);

    check_cancellation!(cancel);
    send_state!(ScanState::CompressToJpeg);

    let raw_image = raw_image(parameters, pixels)?;
    let jpeg = encode_jpeg(raw_image, config.scan.page_quality);

    send_state!(ScanState::Done(jpeg));

    Ok(true)
}

#[cfg(feature = "scan")]
#[rustfmt::skip]
fn setup_scanner(scanner: &mut Scanner, name: &str, config: &Config, dpi: u16) {
    log::debug!("Start device setup");

    let options = scanner.options();
    log::debug!("Device options: {options:#?}");

    let values = get_options_values(name, config);
    log::debug!("Options values from config: {values:#?}");

    for (i, option) in options.into_iter().enumerate() {
        let Some(option_name) = option.name.filter(|name| !name.is_empty()) else {
            log::debug!("Skip unnamed option #{i}");
            continue;
        };

        if option_name == "resolution" {
            set_option_value_or_use_default(&option, &OptionValue::Int(dpi as i32));
        } else if let Some(value) = values.get(option_name) {
            set_option_value_or_use_default(&option, value);
        } else {
            log::debug!("Value for option '{option_name}' is not specified. Trying to use a default value");
            set_option_default_value(&option);
        }
    }
}

#[cfg(feature = "scan")]
fn get_options_values<'c>(
    device_name: &str,
    config: &'c Config,
) -> HashMap<BString, OptionValue<'c>> {
    let mut values = HashMap::new();

    for (name, value) in &config.scan.common_options {
        values.insert(name.to_owned(), OptionValue::String(value.as_ref()));
    }

    if let Some(device_specific_values) = config.scanner.get(device_name) {
        for (name, value) in device_specific_values {
            values.insert(name.to_owned(), OptionValue::String(value.as_ref()));
        }
    }

    values
}

#[cfg(feature = "scan")]
fn set_option_value_or_use_default(option: &ScannerOption, value: &OptionValue) {
    let option_name = option.name.unwrap_or_else(|| BStr::new(b"noname"));

    if let Err(err) = option.set_value(value) {
        log::warn!("Failed to set option '{option_name}' to value '{value:?}': {err}. Trying to use a default value");
    } else {
        log::debug!("Successfully set option '{option_name}' to value '{value:?}'");
        return;
    }

    set_option_default_value(option);
}

#[cfg(feature = "scan")]
fn set_option_default_value(option: &ScannerOption) {
    let option_name = option.name.unwrap_or_else(|| BStr::new(b"noname"));

    if !option.is_auto_settable() {
        log::debug!("Option '{option_name}' doesn't have default value");
        return;
    }

    if let Err(err) = option.set_auto() {
        log::warn!("Failed to set '{option_name}' to default value: {err}");
    } else {
        log::debug!("Successfully set option '{option_name}' to auto value");
    }
}

#[cfg(feature = "scan")]
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

#[cfg(feature = "scan")]
fn encode_jpeg(image: libjpeg::RawImage, output_quality: u8) -> Jpeg {
    let bytes = libjpeg::compress_to_jpeg(&image, output_quality);

    Jpeg {
        bytes,
        format: match image.format {
            libjpeg::RawImageFormat::Rgb => JpegFormat::Rgb,
            libjpeg::RawImageFormat::Gray => JpegFormat::Gray,
        },
        width: image.width,
        height: image.height,
    }
}
