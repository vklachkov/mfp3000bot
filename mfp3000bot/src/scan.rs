use anyhow::{bail, Context};
use lazy_static::lazy_static;
use simple_sane::{Device, Parameters, Sane, Scanner};
use std::{
    io::{Cursor, Read},
    thread,
};
use tokio::sync::{mpsc, oneshot};

lazy_static! {
    static ref SANE: Sane = Sane::new().expect("SANE should be initialize successfully");
    static ref DEVICE: Device<'static> = Device::get_first(&SANE)
        .expect("SANE backend should returns devices without error")
        .expect("no scanners");
}

pub enum ScanState {
    Prepair,
    Progress(u8),
    Done(Vec<u8>),
    Error(anyhow::Error),
    Cancelled,
}

pub fn scan(mut cancel: oneshot::Receiver<()>) -> mpsc::Receiver<ScanState> {
    let (mut state_tx, state_rx) = mpsc::channel(4);

    thread::Builder::new()
        .name("scan".to_owned())
        .spawn(move || {
            match scan_page(&mut state_tx, &mut cancel) {
                Ok(Some((parameters, raw))) => {
                    match encode_jpeg(parameters, raw, 75) {
                        Ok(jpeg) => _ = state_tx.blocking_send(ScanState::Done(jpeg)),
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

    check_cancellation!(cancel);
    let mut scanner = Scanner::new(*DEVICE).context("opening device")?;

    // Setup scanner
    //

    check_cancellation!(cancel);
    let mut reader = scanner.start().context("starting scan")?;

    check_cancellation!(cancel);
    let parameters = reader.get_parameters().context("getting parameters")?;

    let page_size = parameters.bytes_per_line * parameters.lines;
    let mut page = vec![0u8; page_size];
    let mut page_offset = 0;

    send_state!(ScanState::Progress(0));

    let mut previous_progress = 0;
    loop {
        const WINDOW_SIZE: usize = 24 * 1024;

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

    match parameters.depth {
        1 => {
            GrayImage::from_raw(width, height, raw)
                .context("creating image from scanner buffer")?
                .write_to(&mut Cursor::new(&mut image), format)
                .unwrap();
        }
        3 => {
            RgbImage::from_raw(width, height, raw)
                .expect("creating image from scanner buffer")
                .write_to(&mut Cursor::new(&mut image), format)
                .unwrap();
        }
        depth => bail!("unsupported image depth {depth}"),
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
