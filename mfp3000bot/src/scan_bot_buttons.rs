use once_cell::sync::Lazy;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

type Action = &'static str;
type Row = usize;
type MessageKey = &'static str;
type Buttons<const N: usize> = Lazy<[(Action, (Row, MessageKey)); N]>;

#[rustfmt::skip]
pub static SCAN_MODE_BUTTONS: Buttons<2> = Lazy::new(|| {
    [
        (ScanMode::SinglePage.into(), (0, "scan_single_page")),
        (ScanMode::Document.into(), (1, "scan_document")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanMode {
    SinglePage,
    Document,
}

#[rustfmt::skip]
pub static SCAN_ACTIONS_BUTTONS: Buttons<3> = Lazy::new(|| {
    [
        (ScanAction::Cancel.into(), (0, "scan_cancel")),
        (ScanAction::Scan.into(), (1, "scan_start")),
        (ScanAction::Preview.into(), (1, "scan_preview")),
    ]
});

#[rustfmt::skip]
pub static DOCUMENT_ACTION_BUTTONS: Buttons<4> = Lazy::new(|| {
    [
        (ScanAction::Cancel.into(), (0, "scan_cancel")),
        (ScanAction::Scan.into(), (1, "scan_add_page")),
        (ScanAction::Preview.into(), (1, "scan_preview_page")),
        (ScanAction::Done.into(), (2, "scan_done")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanAction {
    Done,
    Scan,
    Preview,
    Cancel,
}

#[rustfmt::skip]
pub static SCAN_CANCEL: Buttons<1> = Lazy::new(|| {
    [
        (ScanCancel::Forget.into(), (0, "scan_cancel")),
    ]
});

#[rustfmt::skip]
pub static SCAN_CANCEL_CONFIRM_BUTTONS: Buttons<2> = Lazy::new(|| {
    [
        (ScanCancel::Forget.into(), (0, "scan_cancel_yes")),
        (ScanCancel::Cancel.into(), (0, "scan_cancel_no")),
    ]
});

#[derive(Clone, Copy, strum::Display, strum::IntoStaticStr, strum::EnumString)]
pub enum ScanCancel {
    Forget,
    Cancel,
}

#[rustfmt::skip]
pub static RENAME_DOCUMENT_BUTTONS: Buttons<1> = Lazy::new(|| {
    [
        ("-", (0, "rename_default")),
    ]
});


