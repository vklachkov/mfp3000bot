use once_cell::sync::Lazy;
use std::collections::HashMap;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const PRINT_COMMAND_TEXT: &str =
    "🖨️ Для печати документа просто отправьте PDF или DOCX файл в этот чат!";

pub const NO_PRINTER_IN_CFG: &str =
    "🖨️ Принтер не указан в конфиге. Измените конфигурационный файл и перезапустите бота!";

pub const SUCCESSFUL_PRINT: &dyn Fn(&str) -> String =
    &|doc_name| format!("📄 Документ \"{doc_name}\" успешно отправлен на печать!");

pub const FAILED_TO_PRINT: &dyn Fn(&str) -> String =
    &|doc_name| format!("⚠️ Ошибка печати документа \"{doc_name}\"!");

pub const SELECT_SCAN_MODE: &str = "Выберите количество страниц в документе";

#[rustfmt::skip]
pub static SCAN_MODE_BUTTONS: Lazy<HashMap<&str, (usize, &str)>> = Lazy::new(|| {
    HashMap::from([
        (ScanMode::SinglePage.into(), (0, "📄 Одна страница")),
        (ScanMode::Document.into(), (1, "📕 Многостраничный документ")),
    ])
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanMode {
    SinglePage,
    Document,
}

pub const SELECT_SCAN_ACTION: &str = "Выберите действие";

#[rustfmt::skip]
pub static SCAN_ACTIONS_BUTTONS: Lazy<HashMap<&str, (usize, &str)>> = Lazy::new(|| {
    HashMap::from([
        (ScanAction::Cancel.into(), (0, "⛔ Прервать сканирование")),
        (ScanAction::Scan.into(), (1, "▶️ Начать")),
        (ScanAction::Preview.into(), (1, "👀 Превью")),
    ])
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanAction {
    Scan,
    Preview,
    Cancel,
}

pub const SCAN_PREPAIR: &str = "Подготовка к сканированию...";

pub const SCAN_PROGRESS: &dyn Fn(u8) -> String =
    &|progress| format!("Прогресс сканирования: {progress}%");

pub const SCAN_PREVIEW_DONE: &str = "Превью страницы:";

pub const SCAN_ERROR: &str = "Ошибка сканирования";

pub const SCAN_CANCELLED: &str = "😔 Сканирование отменено";

pub const UNIMPLEMENTED: &str = "🥺 Простите, эта функция ещё не реализована!";

pub const INVALID_STATE: &str = "🐞 Вы нашли баг! Бот находится в некорректном состоянии";

pub fn buttons_to_inline_keyboard(buttons: &HashMap<&str, (usize, &str)>) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new((0..buttons.len()).map(|idx| {
        buttons
            .iter()
            .filter(move |(_, (row, _))| *row == idx)
            .map(|(id, (_, text))| InlineKeyboardButton::callback(*text, *id))
    }))
}
