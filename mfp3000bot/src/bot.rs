use crate::{
    bot_data::*,
    bot_utils::*,
    config::Config,
    pdf_builder::PdfBuilder,
    print,
    scan::{self, Jpeg, ScanState},
};
use reqwest::Url;
use std::{future::Future, io, str::FromStr, sync::Arc};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    types::{Document, InputFile},
    utils::command::BotCommands,
};
use tokio::sync::{oneshot, Mutex};

pub type BotDialogue = Dialogue<BotState, InMemStorage<BotState>>;

pub struct Globals {
    config: Config,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommand {
    Start,
    Help,
    Print,
    Scan,
}

#[derive(Clone, Default)]
pub enum BotState {
    #[default]
    Empty,

    /// Выбор режима сканирования: страницу или многостраничный документ.  
    SelectScanMode { dialogue_message: Message },

    /// Выбор первого действия при сканировании.
    /// Это состояние универсально для всех режимов сканирования.
    SelectFirstScanAction {
        dialogue_message: Message,
        mode: ScanMode,
    },

    /// Сканирование страницы.
    /// Это состояние универсально для всех режимов сканирования.
    ScanningPage { cancel: ScanCancellationToken },

    /// Получение имени для отсканированной страницы.
    ReceiveScannedPageName {
        dialogue_message: Message,
        page: Page,
    },

    /// Выбор действия для документа.
    SelectDocumentAction {
        dialogue_message: Message,
        pages: Pages,
    },

    /// Подтверждение отмены сканирования и удаления отсканированных страниц.
    ConfirmDropScannedDocument {
        dialogue_message: Message,
        pages: Pages,
    },

    /// Получение имени для отсканированного документа.
    ReceiveScannedDocumentName {
        dialogue_message: Message,
        pages: Pages,
    },
}

pub type ScanCancellationToken = Arc<Mutex<Option<oneshot::Sender<()>>>>;
pub type Page = Jpeg;
pub type Pages = Vec<Jpeg>;

enum ScanResult {
    Done(Page),
    Cancelled,
    Error(anyhow::Error),
}

pub async fn start(config: Config) {
    let bot = Bot::new(&config.telegram.token);

    let globals = Arc::new(Globals { config });

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<BotState>::new(), globals])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<anyhow::Error> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<BotCommand, _>()
        .branch(
            case![BotState::Empty]
                .branch(case![BotCommand::Start].endpoint(hello))
                .branch(case![BotCommand::Help].endpoint(help))
                .branch(case![BotCommand::Print].endpoint(print_document_help))
                .branch(case![BotCommand::Scan].endpoint(start_scan)),
        )
        .endpoint(bot_busy);

    let message_handler = Update::filter_message()
        .filter_async(filter_users)
        .branch(command_handler)
        .branch(dptree::filter(|msg: Message| msg.document().is_some()).endpoint(print_document))
        .branch(
            case![BotState::ReceiveScannedPageName {
                dialogue_message,
                page
            }]
            .endpoint(receive_page_name),
        )
        .branch(
            case![BotState::ReceiveScannedDocumentName {
                dialogue_message,
                pages
            }]
            .endpoint(receive_document_name),
        );

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![BotState::SelectScanMode { dialogue_message }].endpoint(select_scan_mode))
        .branch(
            case![BotState::SelectFirstScanAction {
                dialogue_message,
                mode
            }]
            .endpoint(first_scan_action_selected),
        )
        .branch(
            case![BotState::SelectDocumentAction {
                dialogue_message,
                pages
            }]
            .endpoint(receive_multipage_scan_action_selection),
        )
        .branch(case![BotState::ScanningPage { cancel }].endpoint(receive_scan_cancellation))
        .branch(
            case![BotState::ConfirmDropScannedDocument {
                dialogue_message,
                pages
            }]
            .endpoint(receive_scan_cancel_confirmation),
        )
        .branch(
            case![BotState::ReceiveScannedPageName {
                dialogue_message,
                page
            }]
            .endpoint(receive_page_rename_cancel),
        )
        .branch(
            case![BotState::ReceiveScannedDocumentName {
                dialogue_message,
                pages
            }]
            .endpoint(receive_document_rename_cancel),
        );

    dialogue::enter::<Update, InMemStorage<BotState>, BotState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

/// Фильтр по белому списку пользователей.
///
/// К сожалению, Telegram не даёт скрыть бота из поиска и приходится делать фильтр.
async fn filter_users(globals: Arc<Globals>, bot: Bot, message: Message) -> bool {
    let Some(username) = message.from().and_then(|from| from.username.as_ref()) else {
        return false;
    };

    let allow = globals.config.telegram.allowed_users.contains(username);
    if !allow {
        log::info!("Unallowed user {username} is trying to access bot");
        _ = bot.send_message(message.chat.id, UNALLOWED_USER).await;
    }

    allow
}

/// Команда `/start`.
async fn hello(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, HELLO).await
}

/// Команда `/help`.
async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, HELP).await
}

/// Любая команда, когда бот не находится в состоянии Empty.
async fn bot_busy(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, BOT_BUSY).await
}

/// Команда `/print`.
async fn print_document_help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, PRINT_COMMAND_TEXT).await
}

/// Отправка документа в чат.
async fn print_document(globals: Arc<Globals>, bot: Bot, msg: Message) -> anyhow::Result<()> {
    let Some(printer) = globals.config.devices.printer.as_deref() else {
        return send_msg(&bot, msg.chat.id, NO_PRINTER_IN_CFG).await;
    };

    let document = msg
        .document()
        .expect("Message must have document attachment");

    let Some((document_name, document_url)) = get_document(&globals, &bot, document).await? else {
        return send_msg(&bot, msg.chat.id, UNSUPPORTED_DOCUMENT).await;
    };

    match print::print_remote_file(printer, &document_name, &document_url) {
        Ok(()) => {
            log::debug!("Document '{document_name}' successfully printed");
            send_msg(&bot, msg.chat.id, &SUCCESSFUL_PRINT(&document_name)).await?;
        }

        // TODO: Отправлять в сообщение человекочитаемую ошибку печати.
        Err(err) => {
            log::error!("Failed to print document '{document_name}': {err:#}");
            send_msg(&bot, msg.chat.id, &FAILED_TO_PRINT(&document_name)).await?;
        }
    }

    Ok(())
}

/// Запрашивает информацию о файле, проверяет расширение и
/// возвращает имя документа и ссылку на него.
///
/// Возвращает None, если документ не поддерживается.
async fn get_document(
    globals: &Globals,
    bot: &Bot,
    doc: &Document,
) -> anyhow::Result<Option<(String, Url)>> {
    let file = bot.get_file(&doc.file.id).await?;

    let Some(file_name) = doc.file_name.clone() else {
        return Ok(None);
    };

    let lowercase_file_name = file_name.to_lowercase();
    if !lowercase_file_name.ends_with(".pdf")
        && !lowercase_file_name.ends_with(".docx")
        && !lowercase_file_name.ends_with(".txt")
    {
        return Ok(None);
    }

    // TODO: Оформить PR в Teloxide и убрать ручную сборку ссылки.
    let token = &globals.config.telegram.token;
    let file_path = file.path;
    let file_url = bot
        .api_url()
        .join(&format!("file/bot{token}/{file_path}"))
        .expect("url should be valid");

    Ok(Some((file_name, file_url)))
}

/// Команда `/scan`.
async fn start_scan(bot: Bot, dialogue: BotDialogue) -> anyhow::Result<()> {
    let dialogue_message =
        send_interative(&bot, &dialogue, SELECT_SCAN_MODE, &*SCAN_MODE_BUTTONS).await?;

    dialogue
        .update(BotState::SelectScanMode { dialogue_message })
        .await?;

    Ok(())
}

async fn select_scan_mode(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    dialogue_message: Message, // From `State::SelectScanMode`.
) -> anyhow::Result<()> {
    let Some(mode) = q.data else {
        return Ok(());
    };

    let Ok(mode) = ScanMode::from_str(&mode) else {
        panic!("Invalid scan mode '{mode}'");
    };

    show_scan_action_selector(bot, dialogue, Some(dialogue_message), mode).await?;

    Ok(())
}

async fn show_scan_action_selector(
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Option<Message>,
    mode: ScanMode,
) -> anyhow::Result<()> {
    let dialogue_message = if let Some(message) = dialogue_message {
        edit_interative(&bot, &message, SELECT_SCAN_ACTION, &*SCAN_ACTIONS_BUTTONS).await?
    } else {
        send_interative(&bot, &dialogue, SELECT_SCAN_ACTION, &*SCAN_ACTIONS_BUTTONS).await?
    };

    dialogue
        .update(BotState::SelectFirstScanAction {
            dialogue_message,
            mode,
        })
        .await?;

    Ok(())
}

async fn first_scan_action_selected(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    (dialogue_message, mode): (Message, ScanMode), // From `State::SelectFirstScanAction`.
) -> anyhow::Result<()> {
    let Some(action) = q.data else {
        return Ok(());
    };

    let Ok(action) = ScanAction::from_str(&action) else {
        panic!("Invalid scan action '{action}'");
    };

    match action {
        ScanAction::Scan => {
            scan_first_page(globals, bot, dialogue, dialogue_message, mode).await?;
        }
        ScanAction::Preview => {
            scan_first_page_preview(globals, bot, dialogue, dialogue_message, mode).await?;
        }
        ScanAction::Cancel => {
            edit_msg(&bot, &dialogue_message, SCAN_CANCELLED).await?;
            dialogue.update(BotState::Empty).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn scan_first_page(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    mode: ScanMode,
) -> anyhow::Result<()> {
    let (cancel_tx, cancel_rx) = oneshot::channel();

    dialogue
        .update(BotState::ScanningPage {
            cancel: Arc::new(Mutex::new(Some(cancel_tx))),
        })
        .await?;

    tokio::spawn(async move {
        if let Err(err) =
            scan_first_page_task(globals, bot, dialogue, dialogue_message, mode, cancel_rx).await
        {
            log::error!("Telegram error: {err:#}");
        }
    });

    Ok(())
}

async fn scan_first_page_task(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    mode: ScanMode,
    cancel: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let dpi = globals.config.scanner_common.page_dpi;
    let scan_result = scan_page(globals, &bot, &dialogue_message, dpi, cancel).await?;
    match scan_result {
        ScanResult::Done(page) => match mode {
            ScanMode::SinglePage => {
                show_rename_page_dialog(bot, dialogue, dialogue_message, page).await?;
            }
            ScanMode::Document => {
                show_document_action_selector(bot, dialogue, Some(dialogue_message), vec![page])
                    .await?;
            }
        },
        ScanResult::Cancelled => {
            show_scan_action_selector(bot, dialogue, Some(dialogue_message), mode).await?;
        }
        ScanResult::Error(err) => {
            // TODO: Отправка человекочитаемой ошибки в сообщении.
            log::error!("Ошибка сканирования: {err:#}");
            edit_msg(&bot, &dialogue_message, SCAN_ERROR).await?;
            show_scan_action_selector(bot, dialogue, None, mode).await?;
        }
    }

    Ok(())
}

async fn scan_first_page_preview(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    scan_mode: ScanMode,
) -> anyhow::Result<()> {
    let (cancel_tx, cancel_rx) = oneshot::channel();

    dialogue
        .update(BotState::ScanningPage {
            cancel: Arc::new(Mutex::new(Some(cancel_tx))),
        })
        .await?;

    tokio::spawn(async move {
        if let Err(err) = preview_page_task(
            globals,
            bot,
            dialogue,
            dialogue_message,
            cancel_rx,
            move |bot, dialogue, message| {
                show_scan_action_selector(bot, dialogue, message, scan_mode)
            },
        )
        .await
        {
            log::error!("Telegram error: {err:#}");
        }
    });

    Ok(())
}

async fn preview_page_task<Fn, F>(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    cancel: oneshot::Receiver<()>,
    update_message: Fn,
) -> anyhow::Result<()>
where
    Fn: FnOnce(Bot, BotDialogue, Option<Message>) -> F,
    F: Future<Output = anyhow::Result<()>>,
{
    let dpi = globals.config.scanner_common.preview_dpi;
    let scan_result = scan_page(globals, &bot, &dialogue_message, dpi, cancel).await?;
    match scan_result {
        ScanResult::Done(jpeg) => {
            edit_msg(&bot, &dialogue_message, SCAN_PREVIEW_DONE).await?;

            bot.send_photo(dialogue.chat_id(), InputFile::memory(jpeg.bytes))
                .await?;

            update_message(bot, dialogue, None).await?
        }
        ScanResult::Cancelled => {
            update_message(bot, dialogue, Some(dialogue_message)).await?;
        }
        ScanResult::Error(err) => {
            // TODO: Отправка человекочитаемой ошибки в сообщении.
            log::error!("Ошибка сканирования: {err:#}");
            edit_msg(&bot, &dialogue_message, SCAN_ERROR).await?;

            update_message(bot, dialogue, None).await?;
        }
    }

    Ok(())
}

/// Читает изображение из сканера, отображая состояние сканирования в сообщении.
///
/// Возвращает ошибку только в случае сбоя Telegram.
async fn scan_page(
    globals: Arc<Globals>,
    bot: &Bot,
    message: &Message,
    dpi: u16,
    cancel: oneshot::Receiver<()>,
) -> anyhow::Result<ScanResult> {
    let mut state_receiver = scan::start(globals.config.clone(), dpi, cancel);
    while let Some(state) = state_receiver.recv().await {
        match state {
            ScanState::Prepair => {
                edit_interative(bot, message, SCAN_PREPAIR, &*SCAN_CANCEL).await?;
            }
            ScanState::Progress(p) => {
                edit_interative(bot, message, &SCAN_PROGRESS(p), &*SCAN_CANCEL).await?;
            }
            ScanState::CompressToJpeg => {
                edit_msg(bot, message, SCAN_COMPRESS_JPEG).await?;
            }
            ScanState::Done(jpeg) => {
                return Ok(ScanResult::Done(jpeg));
            }
            ScanState::Error(err) => {
                return Ok(ScanResult::Error(err));
            }
            ScanState::Cancelled => {
                edit_msg(bot, message, SCAN_CANCELLED).await?;
            }
        };
    }

    Ok(ScanResult::Cancelled)
}

async fn receive_scan_cancellation(
    q: CallbackQuery,
    cancel: ScanCancellationToken, // From `State::ScanningPage`.
) -> anyhow::Result<()> {
    if q.data.is_none() {
        return Ok(());
    };

    let Some(cancel) = cancel.lock().await.take() else {
        return Ok(());
    };

    _ = cancel.send(());

    Ok(())
}

async fn show_rename_page_dialog(
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    page: Page,
) -> anyhow::Result<()> {
    edit_interative(
        &bot,
        &dialogue_message,
        RENAME_DOCUMENT,
        &*RENAME_DOCUMENT_BUTTONS,
    )
    .await?;

    dialogue
        .update(BotState::ReceiveScannedPageName {
            dialogue_message,
            page,
        })
        .await?;

    Ok(())
}

async fn receive_page_name(
    bot: Bot,
    dialogue: BotDialogue,
    msg: Message,
    (dialogue_message, page): (Message, Page), // From `State::ReceiveScannedPageName`.
) -> anyhow::Result<()> {
    let Some(name) = msg.text() else {
        return send_msg(&bot, msg.chat.id, INVALID_DOCUMENT_NAME).await;
    };

    edit_msg(&bot, &dialogue_message, RENAME_DOCUMENT).await?;

    send_page(&bot, dialogue.chat_id(), None, name, page).await?;

    dialogue.update(BotState::Empty).await?;

    Ok(())
}

async fn receive_page_rename_cancel(
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, page): (Message, Page), // From `State::ReceiveScannedPageName`.
) -> anyhow::Result<()> {
    send_page(
        &bot,
        dialogue.chat_id(),
        Some(dialogue_message),
        DEFAULT_SINGLE_PAGE_NAME,
        page,
    )
    .await?;

    dialogue.update(BotState::Empty).await?;

    Ok(())
}

async fn send_page(
    bot: &Bot,
    chat_id: ChatId,
    dialogue_message: Option<Message>,
    name: &str,
    page: Page,
) -> anyhow::Result<()> {
    if let Some(dialogue_message) = dialogue_message {
        edit_msg(bot, &dialogue_message, SINGLE_PAGE_SCAN_RESULT).await?;
    } else {
        send_msg(bot, chat_id, SINGLE_PAGE_SCAN_RESULT).await?;
    }

    let document = InputFile::memory(page.bytes.to_owned()).file_name(format!("{name}.jpg"));
    bot.send_document(chat_id, document).await?;

    Ok(())
}

async fn show_document_action_selector(
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Option<Message>,
    pages: Vec<Jpeg>,
) -> anyhow::Result<()> {
    let dialogue_message = if let Some(message) = dialogue_message {
        edit_interative(
            &bot,
            &message,
            &SELECT_DOCUMENT_ACTION(pages.len()),
            &*DOCUMENT_ACTION_BUTTONS,
        )
        .await?
    } else {
        send_interative(
            &bot,
            &dialogue,
            SELECT_SCAN_ACTION,
            &*DOCUMENT_ACTION_BUTTONS,
        )
        .await?
    };

    dialogue
        .update(BotState::SelectDocumentAction {
            dialogue_message,
            pages,
        })
        .await?;

    Ok(())
}

async fn receive_multipage_scan_action_selection(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    (dialogue_message, pages): (Message, Pages), // From `State::SelectDocumentAction`.
) -> anyhow::Result<()> {
    let Some(action) = q.data else {
        return Ok(());
    };

    let Ok(action) = ScanAction::from_str(&action) else {
        panic!("Invalid scan action '{action}'");
    };

    match action {
        ScanAction::Done => {
            show_rename_document_dialog(bot, dialogue, dialogue_message, pages).await?;
        }
        ScanAction::Scan => {
            scan_document_page(globals, bot, dialogue, (dialogue_message, pages)).await?;
        }
        ScanAction::Preview => {
            scan_document_page_preview(globals, bot, dialogue, (dialogue_message, pages)).await?;
        }
        ScanAction::Cancel => {
            ask_scan_cancel_confirmation(bot, dialogue, (dialogue_message, pages)).await?;
        }
    }

    Ok(())
}

async fn scan_document_page(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, pages): (Message, Pages), // From `State::SelectDocumentAction`.
) -> anyhow::Result<()> {
    let (cancel_tx, cancel_rx) = oneshot::channel();

    dialogue
        .update(BotState::ScanningPage {
            cancel: Arc::new(Mutex::new(Some(cancel_tx))),
        })
        .await?;

    tokio::spawn(async move {
        if let Err(err) =
            scan_document_page_task(globals, bot, dialogue, dialogue_message, cancel_rx, pages)
                .await
        {
            log::error!("Telegram error: {err:#}");
        }
    });

    Ok(())
}

async fn scan_document_page_task(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    cancel: oneshot::Receiver<()>,
    mut pages: Pages,
) -> anyhow::Result<()> {
    let dpi = globals.config.scanner_common.page_dpi;
    let scan_result = scan_page(globals, &bot, &dialogue_message, dpi, cancel).await?;
    match scan_result {
        ScanResult::Done(page) => {
            pages.push(page);

            show_document_action_selector(bot, dialogue, Some(dialogue_message), pages).await?;
        }
        ScanResult::Cancelled => {
            show_document_action_selector(bot, dialogue, Some(dialogue_message), pages).await?;
        }
        ScanResult::Error(err) => {
            // TODO: Отправка человекочитаемой ошибки в сообщении.
            log::error!("Ошибка сканирования: {err:#}");
            edit_msg(&bot, &dialogue_message, SCAN_ERROR).await?;

            show_document_action_selector(bot, dialogue, None, pages).await?;
        }
    }

    Ok(())
}

async fn ask_scan_cancel_confirmation(
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, pages): (Message, Pages), // From `State::SelectDocumentAction`.
) -> anyhow::Result<()> {
    edit_interative(
        &bot,
        &dialogue_message,
        SCAN_CANCEL_CONFIRMATION,
        &*SCAN_CANCEL_CONFIRM_BUTTONS,
    )
    .await?;

    dialogue
        .update(BotState::ConfirmDropScannedDocument {
            dialogue_message,
            pages,
        })
        .await?;

    Ok(())
}

async fn receive_scan_cancel_confirmation(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    (dialogue_message, pages): (Message, Pages), // From `State::ConfirmDropScannedDocument`.
) -> anyhow::Result<()> {
    let Some(answer) = q.data else {
        return Ok(());
    };

    let Ok(answer) = ScanCancel::from_str(&answer) else {
        panic!("Invalid scan mode '{answer}'");
    };

    match answer {
        ScanCancel::Forget => {
            edit_msg(&bot, &dialogue_message, SCAN_CANCELLED).await?;
            dialogue.update(BotState::Empty).await?;
        }
        ScanCancel::Cancel => {
            show_document_action_selector(bot, dialogue, Some(dialogue_message), pages).await?;
        }
    }

    Ok(())
}

async fn scan_document_page_preview(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, pages): (Message, Pages), // From `State::SelectDocumentAction`.
) -> anyhow::Result<()> {
    let (cancel_tx, cancel_rx) = oneshot::channel();

    dialogue
        .update(BotState::ScanningPage {
            cancel: Arc::new(Mutex::new(Some(cancel_tx))),
        })
        .await?;

    tokio::spawn(async move {
        if let Err(err) = preview_page_task(
            globals,
            bot,
            dialogue,
            dialogue_message,
            cancel_rx,
            move |bot, dialogue, message| {
                show_document_action_selector(bot, dialogue, message, pages)
            },
        )
        .await
        {
            log::error!("Telegram error: {err:#}");
        }
    });

    Ok(())
}

async fn show_rename_document_dialog(
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Message,
    pages: Pages,
) -> anyhow::Result<()> {
    edit_interative(
        &bot,
        &dialogue_message,
        RENAME_DOCUMENT,
        &*RENAME_DOCUMENT_BUTTONS,
    )
    .await?;

    dialogue
        .update(BotState::ReceiveScannedDocumentName {
            dialogue_message,
            pages,
        })
        .await?;

    Ok(())
}

async fn receive_document_rename_cancel(
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, pages): (Message, Pages), // From `State::ReceiveScannedDocumentName`.
) -> anyhow::Result<()> {
    send_pdf(
        &bot,
        &dialogue,
        Some(dialogue_message),
        DEFAULT_DOC_NAME,
        pages,
    )
    .await?;

    dialogue.update(BotState::Empty).await?;

    Ok(())
}

async fn receive_document_name(
    bot: Bot,
    dialogue: BotDialogue,
    msg: Message,
    (dialogue_message, pages): (Message, Pages), // From `State::ReceiveScannedDocumentName`.
) -> anyhow::Result<()> {
    let Some(name) = msg.text() else {
        return send_msg(&bot, msg.chat.id, INVALID_DOCUMENT_NAME).await;
    };

    edit_msg(&bot, &dialogue_message, RENAME_DOCUMENT).await?;

    send_pdf(&bot, &dialogue, None, name, pages).await?;

    dialogue.update(BotState::Empty).await?;

    Ok(())
}

async fn send_pdf(
    bot: &Bot,
    dialogue: &BotDialogue,
    dialogue_message: Option<Message>,
    name: &str,
    pages: Pages,
) -> anyhow::Result<()> {
    let dialogue_message = if let Some(dialogue_message) = dialogue_message {
        bot.edit_message_text(dialogue.chat_id(), dialogue_message.id, SCAN_PREPARE_PDF)
            .await?
    } else {
        bot.send_message(dialogue.chat_id(), SCAN_PREPARE_PDF)
            .await?
    };

    let pdf = tokio::task::spawn_blocking(|| convert_pages_to_document(pages))
        .await
        .unwrap();

    edit_msg(bot, &dialogue_message, MULTIPAGE_SCAN_RESULT).await?;

    bot.send_document(
        dialogue.chat_id(),
        InputFile::memory(pdf).file_name(format!("{name}.pdf")),
    )
    .await?;
    Ok(())
}

fn convert_pages_to_document(pages: Vec<Jpeg>) -> Vec<u8> {
    // TODO: Remove hardcoded dpi
    let pdf_builder = PdfBuilder::new("Document", 300.0);

    for page in pages {
        pdf_builder.add_page(page).unwrap();
    }

    let mut pdf = Vec::new();
    pdf_builder
        .write_to(&mut io::BufWriter::with_capacity(128 * 1024, &mut pdf))
        .unwrap();

    pdf
}
