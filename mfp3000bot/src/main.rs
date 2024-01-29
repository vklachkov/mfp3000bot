mod print;

use print::print_remote_file;
use teloxide::prelude::*;
use tokio::sync::oneshot;

const TOKEN: &str = "6641366668:AAGWTel0IJt1gyt48KBJmLVZvhgXXQHM6AY";

#[tokio::main]
async fn main() {
    simple_logger::init().unwrap();
    log::info!("Starting...");

    let bot = Bot::new(TOKEN);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        log::debug!("Message: {msg:?}");

        let Some(doc) = msg.document() else {
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
