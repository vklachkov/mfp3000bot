use crate::{
    bot_data::{self as msg, buttons_to_inline_keyboard},
    config::Config,
    print,
    scan::{self, ScanState},
};
use reqwest::Url;
use std::{str::FromStr, sync::Arc};
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    types::{Chat, Document, InlineKeyboardMarkup, InputFile},
    utils::command::BotCommands,
};
use tokio::sync::{oneshot, Mutex};

type BotDialogue = Dialogue<BotState, InMemStorage<BotState>>;
pub struct Globals {
    config: Config,
}

#[derive(Clone, Default)]
pub enum BotState {
    #[default]
    Empty,

    ReceiveScanMode {
        dialogue_message: Message,
    },

    ReceiveScanAction {
        dialogue_message: Message,
        mode: msg::ScanMode,
    },

    ScanPreview {
        dialogue_message: Message,
        cancel: ScanCancellationToken,
    },
}

type ScanCancellationToken = Arc<Mutex<Option<oneshot::Sender<()>>>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum BotCommand {
    Help,
    Print,
    Scan,
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
        .branch(case![BotCommand::Help].endpoint(help))
        .branch(
            case![BotState::Empty]
                .branch(case![BotCommand::Print].endpoint(print_hint))
                .branch(case![BotCommand::Scan].endpoint(start_scan)),
        )
        .endpoint(unknown_request);

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::filter(|msg: Message| msg.document().is_some()).endpoint(print_doc))
        .branch(dptree::endpoint(unknown_request));

    let callback_query_handler = Update::filter_callback_query()
        .branch(
            case![BotState::ReceiveScanMode { dialogue_message }]
                .endpoint(receive_scan_mode_selection),
        )
        .branch(
            case![BotState::ReceiveScanAction {
                dialogue_message,
                mode
            }]
            .endpoint(receive_scan_action_selection),
        )
        .branch(
            case![BotState::ScanPreview {
                dialogue_message,
                cancel,
            }]
            .endpoint(receive_scan_preview_cancellation),
        )
        .branch(dptree::endpoint(unknown_request));

    dialogue::enter::<Update, InMemStorage<BotState>, BotState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

async fn print_hint(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, msg::PRINT_COMMAND_TEXT).await
}

async fn print_doc(globals: Arc<Globals>, bot: Bot, msg: Message) -> anyhow::Result<()> {
    let Some(printer) = globals.config.devices.printer.as_deref() else {
        return send_msg(&bot, msg.chat.id, msg::NO_PRINTER_IN_CFG).await;
    };

    let document = msg
        .document()
        .expect("Message must have document attachment");

    let (document_name, document_url) = get_document(&globals, &bot, document).await?;

    match print::print_remote_file(printer, &document_name, &document_url) {
        Ok(()) => {
            log::debug!("Document '{document_name}' successfully printed");
            send_msg(&bot, msg.chat.id, &msg::SUCCESSFUL_PRINT(&document_name)).await?;
        }

        // TODO: Отправлять в сообщение человекочитаемую ошибку печати.
        Err(err) => {
            log::error!("Failed to print document '{document_name}': {err:#}");
            send_msg(&bot, msg.chat.id, &msg::FAILED_TO_PRINT(&document_name)).await?;
        }
    }

    Ok(())
}

async fn get_document(
    globals: &Globals,
    bot: &Bot,
    doc: &Document,
) -> anyhow::Result<(String, Url)> {
    let file = bot.get_file(&doc.file.id).send().await?;
    let file_name = doc
        .file_name
        .as_deref()
        .unwrap_or("(telegram file)")
        .to_owned();

    let token = &globals.config.telegram.token;
    let file_path = file.path;
    let file_url = bot
        .api_url()
        .join(&format!("file/bot{token}/{file_path}"))
        .expect("url should be valid");

    Ok((file_name, file_url))
}

async fn start_scan(bot: Bot, dialogue: BotDialogue, msg: Message) -> anyhow::Result<()> {
    let dialogue_message = bot
        .send_message(msg.chat.id, msg::SELECT_SCAN_MODE)
        .reply_markup(msg::buttons_to_inline_keyboard(&msg::SCAN_MODE_BUTTONS))
        .await?;

    dialogue
        .update(BotState::ReceiveScanMode { dialogue_message })
        .await?;

    Ok(())
}

async fn receive_scan_mode_selection(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    dialogue_message: Message, // From `State::ReceiveScanMode`.
) -> anyhow::Result<()> {
    let Some(mode) = q.data else {
        return Ok(());
    };

    let Ok(mode) = msg::ScanMode::from_str(&mode) else {
        panic!("Invalid scan mode '{mode}'");
    };

    select_scan_action(bot, dialogue, Some(dialogue_message), mode).await?;

    Ok(())
}

async fn select_scan_action(
    bot: Bot,
    dialogue: BotDialogue,
    dialogue_message: Option<Message>,
    mode: msg::ScanMode,
) -> anyhow::Result<()> {
    let actions = msg::buttons_to_inline_keyboard(&msg::SCAN_ACTIONS_BUTTONS);

    let dialogue_message = if let Some(dialogue_message) = dialogue_message {
        bot.edit_message_text(
            dialogue_message.chat.id,
            dialogue_message.id,
            msg::SELECT_SCAN_ACTION,
        )
        .reply_markup(actions)
        .await?
    } else {
        bot.send_message(dialogue.chat_id(), msg::SELECT_SCAN_ACTION)
            .reply_markup(actions)
            .await?
    };

    dialogue
        .update(BotState::ReceiveScanAction {
            dialogue_message,
            mode,
        })
        .await?;

    Ok(())
}

async fn receive_scan_action_selection(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    (dialogue_message, mode): (Message, msg::ScanMode), // From `State::ReceiveScanAction`.
) -> anyhow::Result<()> {
    let Some(action) = q.data else {
        return Ok(());
    };

    let Ok(action) = msg::ScanAction::from_str(&action) else {
        panic!("Invalid scan action '{action}'");
    };

    match action {
        msg::ScanAction::Scan => todo!(),
        msg::ScanAction::Preview => {
            scan_preview(globals, bot, dialogue, (dialogue_message, mode)).await?;
        }
        msg::ScanAction::Cancel => {
            edit_msg(&bot, &dialogue_message, msg::SCAN_CANCELLED).await?;
            dialogue.update(BotState::Empty).await?;
        }
    }

    Ok(())
}

async fn scan_preview(
    globals: Arc<Globals>,
    bot: Bot,
    dialogue: BotDialogue,
    (dialogue_message, mode): (Message, msg::ScanMode), // From `State::ReceiveScanAction`.
) -> anyhow::Result<()> {
    let (cancel_tx, cancel_rx) = oneshot::channel();

    dialogue
        .update(BotState::ScanPreview {
            dialogue_message: dialogue_message.clone(),
            cancel: Arc::new(Mutex::new(Some(cancel_tx))),
        })
        .await?;

    tokio::spawn(async move {
        if let Err(err) = scan_preview_impl(globals, &bot, &dialogue_message, cancel_rx).await {
            log::error!("Telegram error: {err:#}");
        }

        if let Err(err) = select_scan_action(bot, dialogue, None, mode).await {
            log::error!("Telegram error: {err:#}");
        }
    });

    Ok(())
}

async fn scan_preview_impl(
    globals: Arc<Globals>,
    bot: &Bot,
    message: &Message,
    cancel: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let cancel_button = buttons_to_inline_keyboard(&msg::SCAN_CANCEL);

    let mut state_receiver = scan::start(globals.config.clone(), cancel);
    while let Some(state) = state_receiver.recv().await {
        match state {
            ScanState::Prepair => {
                edit_interative(&bot, &message, msg::SCAN_PREPAIR, &cancel_button).await?;
            }
            ScanState::Progress(p) => {
                edit_interative(&bot, &message, &msg::SCAN_PROGRESS(p), &cancel_button).await?;
            }
            ScanState::Done(jpeg) => {
                edit_msg(&bot, &message, msg::SCAN_PREVIEW_DONE).await?;

                bot.send_photo(message.chat.id, InputFile::memory(jpeg.0))
                    .await?;
            }
            ScanState::Error(err) => {
                log::error!("Ошибка сканирования: {err:#}");
                edit_msg(&bot, &message, msg::SCAN_ERROR).await?;
            }
            ScanState::Cancelled => {
                edit_msg(&bot, &message, msg::SCAN_CANCELLED).await?;
            }
        };
    }

    Ok(())
}

async fn receive_scan_preview_cancellation(
    q: CallbackQuery,
    (_, cancel): (Message, ScanCancellationToken), // From `State::ScanPreview`.
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

async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    send_msg(&bot, msg.chat.id, msg::UNIMPLEMENTED).await
}

async fn unknown_request(bot: Bot, dialogue: BotDialogue) -> anyhow::Result<()> {
    bot.send_message(dialogue.chat_id(), msg::INVALID_STATE)
        .await?;

    Ok(())
}

pub async fn send_msg(bot: &Bot, chat_id: ChatId, text: &str) -> anyhow::Result<()> {
    bot.send_message(chat_id, text).await?;
    Ok(())
}

pub async fn send_interative(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    buttons: &InlineKeyboardMarkup,
) -> anyhow::Result<()> {
    bot.send_message(chat_id, text)
        .reply_markup(buttons.to_owned())
        .await?;

    Ok(())
}

pub async fn edit_msg(bot: &Bot, msg: &Message, text: &str) -> anyhow::Result<()> {
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    Ok(())
}

pub async fn edit_interative(
    bot: &Bot,
    msg: &Message,
    text: &str,
    buttons: &InlineKeyboardMarkup,
) -> anyhow::Result<()> {
    bot.edit_message_text(msg.chat.id, msg.id, text)
        .reply_markup(buttons.to_owned())
        .await?;

    Ok(())
}
