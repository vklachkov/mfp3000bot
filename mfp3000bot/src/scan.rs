use lazy_static::lazy_static;
use simple_sane::{Device, Parameters, Sane, Scanner};
use std::{
    io::{BufWriter, Cursor},
    thread,
};
use teloxide::{prelude::*, types::InputFile, RequestError};
use tokio::sync::{mpsc, oneshot};

lazy_static! {
    static ref SANE: Sane = Sane::new().expect("SANE should be initialize successfully");
    static ref DEVICE: Device<'static> = Device::get_first(&SANE)
        .expect("SANE backend should returns devices without error")
        .expect("no scanners");
}

pub async fn demo_scan(bot: Bot, msg: Message) -> Result<(), RequestError> {
    let (parameters_tx, parameters_rx) = oneshot::channel::<Parameters>();
    let (bytes_tx, mut bytes_rx) = mpsc::channel::<Vec<u8>>(4);

    thread::spawn(move || {
        let mut scanner = Scanner::new(*DEVICE).unwrap();
        let mut active_scanner = scanner.start().unwrap();

        let parameters = active_scanner.get_parameters().unwrap();
        if parameters_tx.send(parameters).is_err() {
            return;
        }

        struct Foo {
            bytes_tx: mpsc::Sender<Vec<u8>>,
        }

        impl std::io::Write for Foo {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.bytes_tx.blocking_send(buf.to_vec()).unwrap();
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut foo = Foo { bytes_tx };

        active_scanner.scan(&mut foo, 16 * 1024).unwrap();
    });

    let parameters = parameters_rx.await.expect("TODO");

    let image_size = parameters.bytes_per_line * parameters.lines;
    let mut image_buffer = Vec::with_capacity(image_size);

    let msg = bot
        .send_message(msg.chat.id, progress_message_text(0))
        .await?;

    let mut previous_progress = 0;
    while let Some(chunk) = bytes_rx.recv().await {
        image_buffer.extend(chunk);

        let progress = image_buffer.len() / image_size;
        if progress != 100 && progress - previous_progress >= 5 {
            previous_progress = progress;

            bot.edit_message_text(msg.chat.id, msg.id, progress_message_text(progress))
                .await?;
        }
    }

    bot.edit_message_text(msg.chat.id, msg.id, progress_message_text(100))
        .await?;

    let jpeg = encode_jpeg(parameters, image_buffer, 65).await;
    bot.send_photo(msg.chat.id, InputFile::memory(jpeg)).await?;

    Ok(())
}

fn progress_message_text(progress: usize) -> String {
    format!("Scan progress: {progress}%")
}

async fn encode_jpeg(parameters: Parameters, raw: Vec<u8>, output_quality: u8) -> Vec<u8> {
    let (jpeg_tx, jpeg_rx) = oneshot::channel();

    thread::spawn(move || {
        use image::{GrayImage, ImageOutputFormat, RgbImage};

        let width = parameters.pixels_per_line as u32;
        let height = parameters.lines as u32;

        let mut jpeg = Vec::new();

        match parameters.depth {
            1 => {
                GrayImage::from_raw(width, height, raw)
                    .expect("image should be valid")
                    .write_to(
                        &mut BufWriter::new(Cursor::new(&mut jpeg)),
                        ImageOutputFormat::Jpeg(output_quality),
                    )
                    .expect("should be successful");
            }
            3 => {
                RgbImage::from_raw(width, height, raw)
                    .expect("image should be valid")
                    .write_to(
                        &mut BufWriter::new(Cursor::new(&mut jpeg)),
                        ImageOutputFormat::Jpeg(output_quality),
                    )
                    .expect("should be successful");
            }
            _ => panic!("unsupported"),
        };

        jpeg_tx.send(jpeg).unwrap();
    });

    jpeg_rx.await.unwrap()
}
