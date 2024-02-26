use crate::{bot_messages as msg, config::Config, print};
use reqwest::Url;
use std::sync::Arc;
use teloxide::{
    dispatching::{
        dialogue::{self, InMemStorage},
        UpdateHandler,
    },
    prelude::*,
    types::Document,
    utils::command::BotCommands,
};

type BotDialogue = Dialogue<BotState, InMemStorage<BotState>>;
pub struct Globals {
    config: Config,
}

#[derive(Clone, Default)]
pub enum BotState {
    #[default]
    Empty,
}

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
        .branch(case![BotCommand::Print].endpoint(print_hint))
        .branch(case![BotCommand::Scan].endpoint(scan_doc))
        .branch(case![BotCommand::Help].endpoint(help));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(dptree::filter(|msg: Message| msg.document().is_some()).endpoint(print_doc))
        .branch(dptree::endpoint(unknown_request));

    let callback_query_handler = Update::filter_callback_query();

    dialogue::enter::<Update, InMemStorage<BotState>, BotState, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

async fn print_hint(bot: Bot, msg: Message) -> anyhow::Result<()> {
    msg::send(&bot, &msg, msg::PRINT_COMMAND_TEXT).await
}

async fn print_doc(globals: Arc<Globals>, bot: Bot, msg: Message) -> anyhow::Result<()> {
    let Some(printer) = globals.config.devices.printer.as_deref() else {
        return msg::send(&bot, &msg, msg::NO_PRINTER_IN_CFG).await;
    };

    let document = msg
        .document()
        .expect("Message must have document attachment");

    let (document_name, document_url) = get_document(&globals, &bot, document).await?;

    match print::print_remote_file(printer, &document_name, &document_url) {
        Ok(()) => {
            log::debug!("Document '{document_name}' successfully printed");
            msg::send(&bot, &msg, msg::SUCCESSFUL_PRINT(&document_name)).await?;
        }

        // TODO: Отправлять в сообщение человекочитаемую ошибку печати.
        Err(err) => {
            log::error!("Failed to print document '{document_name}': {err:#}");
            msg::send(&bot, &msg, msg::FAILED_TO_PRINT(&document_name)).await?;
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

async fn scan_doc(bot: Bot, _dialogue: BotDialogue, msg: Message) -> anyhow::Result<()> {
    msg::send(&bot, &msg, msg::UNIMPLEMENTED).await
}

async fn help(bot: Bot, msg: Message) -> anyhow::Result<()> {
    msg::send(&bot, &msg, msg::UNIMPLEMENTED).await
}

async fn unknown_request(bot: Bot, msg: Message) -> anyhow::Result<()> {
    msg::send(&bot, &msg, msg::INVALID_STATE).await
}
