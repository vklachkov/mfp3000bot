use once_cell::sync::Lazy;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub const UNALLOWED_USER: &str =
    "👀 У вас нет доступа к этому Telegram боту. Обратитесь к администратору для получения доступа";

pub const HELLO_TEXT: &str = "\
👋 Добро пожаловать в бот для печати и сканирования!

Чтобы распечатать документ просто отправьте отправьте PDF или DOCX файл в этот чат.
    
Для сканирования документа отправьте команду /scan и следуйте инструкции на экране.

Все команды доступны через кнопку \"Меню\" внизу экрана.
";

pub const HELP_TEXT: &str = "\
🖨️ Этот бот предназначен для быстрого доступа к домашнему принтеру через Telegram.

Чтобы распечатать документ просто отправьте отправьте PDF или DOCX файл в этот чат.

Для сканирования документа отправьте команду /scan и следуйте инструкции на экране.";

pub const BOT_BUSY: &str =
    "🕓 Бот занят сканированием документа. Отправьте команду после завершения сканирования.";

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
pub static SCAN_MODE_BUTTONS: Lazy<[(&str, (usize, &str)); 2]> = Lazy::new(|| {
    [
        (ScanMode::SinglePage.into(), (0, "📄 Одна страница")),
        (ScanMode::Document.into(), (1, "📕 Многостраничный документ")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanMode {
    SinglePage,
    Document,
}

pub const SELECT_SCAN_ACTION: &str = "Выберите действие";

#[rustfmt::skip]
pub static SCAN_ACTIONS_BUTTONS: Lazy<[(&str, (usize, &str)); 3]> = Lazy::new(|| {
    [
        (ScanAction::Cancel.into(), (0, "⛔ Прервать сканирование")),
        (ScanAction::Scan.into(), (1, "🚀 Начать")),
        (ScanAction::Preview.into(), (1, "👀 Превью")),
    ]
});

pub const MULTIPAGE_SELECT_SCAN_ACTION: &dyn Fn(usize) -> String =
    &|count| format!("📄 Страниц в документе: {count}. Выберите действие");

#[rustfmt::skip]
pub static MULTIPAGE_SCAN_ACTIONS_BUTTONS: Lazy<[(&str, (usize, &str)); 4]> = Lazy::new(|| {
    [
        (ScanAction::Cancel.into(), (0, "⛔ Прервать сканирование")),
        (ScanAction::Scan.into(), (1, "🚀 Добавить страницу")),
        (ScanAction::Preview.into(), (1, "👀 Превью страницы")),
        (ScanAction::Done.into(), (2, "📥 Завершить")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanAction {
    Done,
    Scan,
    Preview,
    Cancel,
}

pub const SCAN_PREPAIR: &str = "⚙️ Подготовка к сканированию...";

pub const SCAN_PROGRESS: &dyn Fn(f64) -> String =
    &|progress| format!("⏳ Прогресс сканирования: {progress:.0}%");

pub const SCAN_PREVIEW_DONE: &str = "👀 Превью страницы:";

pub const SINGLE_PAGE_SCAN_RESULT: &str = "📄 Отсканированная страница:";

pub const SCAN_ERROR: &str = "⚠️ Ошибка сканирования";

pub const SCAN_PREPARE_PDF: &str = "⚙️ Подготовка PDF документа...";

pub const MULTIPAGE_SCAN_RESULT: &str = "📕 Отсканированный документ:";

#[rustfmt::skip]
pub static SCAN_CANCEL: Lazy<[(&str, (usize, &str)); 1]> = Lazy::new(|| {
    [
        (ScanCancel::Forget.into(), (0, "⛔ Прервать сканирование")),
    ]
});

pub const SCAN_CANCELLED: &str = "👍 Сканирование отменено";

pub const SCAN_CANCEL_CONFIRMATION: &str = "⚠️ Отменить сканирование и удалить документ?";

#[rustfmt::skip]
pub static SCAN_CANCEL_CONFIRM_BUTTONS: Lazy<[(&str, (usize, &str)); 2]> = Lazy::new(|| {
    [
        (ScanCancel::Forget.into(), (0, "🗑️ Да")),
        (ScanCancel::Cancel.into(), (0, "↩️ Нет")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanCancel {
    Forget,
    Cancel,
}

pub fn buttons_to_inline_keyboard(buttons: &[(&str, (usize, &str))]) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new((0..buttons.len()).map(|idx| {
        buttons
            .iter()
            .filter(move |(_, (row, _))| *row == idx)
            .map(|(id, (_, text))| InlineKeyboardButton::callback(*text, *id))
    }))
}
