mod print;

use print::print_remote_file;
use std::{
    io::{BufWriter, Cursor},
    thread,
};
use teloxide::{prelude::*, types::InputFile, ApiError, RequestError};
use tokio::sync::{mpsc, oneshot};

const TOKEN: &str = "6641366668:AAGWTel0IJt1gyt48KBJmLVZvhgXXQHM6AY";

#[tokio::main]
async fn main() {
    simple_logger::init().unwrap();
    log::info!("Starting...");

    let bot = Bot::new(TOKEN);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        log::debug!("Message: {msg:?}");

        let Some(doc) = msg.document() else {
            use simple_sane::{Device, Parameters, Sane, Scanner};

            struct Chunk {
                width: usize,
                height: usize,
                depth: usize,
                bytes: Vec<u8>,
            }

            let (tx, mut rx) = mpsc::unbounded_channel::<Chunk>();

            thread::spawn(move || {
                let sane = Sane::new().unwrap();
                let device = Device::get_first(&sane).unwrap().unwrap();
                let mut scanner = Scanner::new(device).unwrap();
                let mut active_scanner = scanner.start().unwrap();

                let scan_params = active_scanner.get_parameters().unwrap();

                struct Foo {
                    scan_params: Parameters,
                    tx: mpsc::UnboundedSender<Chunk>,
                }

                impl std::io::Write for Foo {
                    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                        self.tx
                            .send(Chunk {
                                width: self.scan_params.pixels_per_line,
                                height: self.scan_params.lines,
                                depth: self.scan_params.bytes_per_line
                                    / self.scan_params.pixels_per_line,
                                bytes: buf.to_vec(),
                            })
                            .unwrap();

                        Ok(buf.len())
                    }

                    fn flush(&mut self) -> std::io::Result<()> {
                        Ok(())
                    }
                }

                let mut foo = Foo { scan_params, tx };

                active_scanner.scan(&mut foo, 16 * 1024).unwrap();
            });

            fn progress(p: f64) -> String {
                println!("PROGRESS!!! {p}");
                format!("Scan progress: {p:.0}%")
            }

            let msg = bot.send_message(msg.chat.id, progress(0.)).await?;

            let mut image = Vec::new();
            let mut width = 0;
            let mut height = 0;
            let mut pp = 0.;

            while let Some(chunk) = rx.recv().await {
                image.extend(chunk.bytes);

                width = chunk.width;
                height = chunk.height;

                let p = (image.len() as f64 / (width * height * chunk.depth) as f64) * 100.;
                if p - pp < 5.0 {
                    continue;
                }

                pp = p;

                let aaa = bot
                    .edit_message_text(msg.chat.id, msg.id, progress(p))
                    .await;

                match aaa {
                    Ok(_) => {}
                    Err(RequestError::Api(ApiError::MessageNotModified)) => {}
                    Err(err) => return Err(err),
                }
            }

            let aaa = bot
                .edit_message_text(msg.chat.id, msg.id, progress(100.))
                .await;

            match aaa {
                Ok(_) => {}
                Err(RequestError::Api(ApiError::MessageNotModified)) => {}
                Err(err) => return Err(err),
            }

            let (jpeg_tx, jpeg_rx) = oneshot::channel();

            thread::spawn(move || {
                let raw_image =
                    image::RgbImage::from_raw(width as u32, height as u32, image).unwrap();

                let mut jpeg_image = Vec::new();
                raw_image
                    .write_to(
                        &mut BufWriter::new(Cursor::new(&mut jpeg_image)),
                        image::ImageOutputFormat::Jpeg(75),
                    )
                    .unwrap();

                jpeg_tx.send(jpeg_image).unwrap();
            });

            let jpeg_image = jpeg_rx.await.unwrap();
            println!("Image size: {}", jpeg_image.len());

            bot.send_photo(msg.chat.id, InputFile::memory(jpeg_image))
                .await?;

            return Ok(());
        };

        let file = bot.get_file(&doc.file.id).send().await?;
        let file_name = doc.file_name.as_deref().unwrap_or("(telegram file)");

        let file_path = file.path;
        let file_url = bot
            .api_url()
            .join(&format!("file/bot{TOKEN}/{file_path}"))
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
    })
    .await;
}
