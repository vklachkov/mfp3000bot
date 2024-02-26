use teloxide::{requests::Requester, types::Message, Bot};

pub const PRINT_COMMAND_TEXT: &str =
    "🖨️ Для печати документа просто отправьте PDF или DOCX файл в этот чат!";

pub const NO_PRINTER_IN_CFG: &str =
    "🖨️ Принтер не указан в конфиге. Измените конфигурационный файл и перезапустите бота!";

pub const SUCCESSFUL_PRINT: &dyn Fn(&str) -> String =
    &|doc_name| format!("📄 Документ \"{doc_name}\" успешно отправлен на печать!");

pub const FAILED_TO_PRINT: &dyn Fn(&str) -> String =
    &|doc_name| format!("⚠️ Ошибка печати документа \"{doc_name}\"!");

pub const UNIMPLEMENTED: &str = "🥺 Простите, эта функция ещё не реализована!";

pub const INVALID_STATE: &str = "🐞 Вы нашли баг! Бот находится в некорректном состоянии";

pub async fn send(bot: &Bot, msg: &Message, text: impl AsRef<str>) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, text.as_ref()).await?;
    Ok(())
}
