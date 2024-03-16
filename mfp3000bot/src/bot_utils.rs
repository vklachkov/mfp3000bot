use crate::{bot::BotDialogue, bot_data::buttons_to_inline_keyboard};
use teloxide::prelude::*;

#[inline(always)]
pub async fn send_msg(bot: &Bot, chat_id: ChatId, text: &str) -> anyhow::Result<()> {
    bot.send_message(chat_id, text).await?;
    Ok(())
}

#[inline(always)]
pub async fn edit_msg(bot: &Bot, msg: &Message, text: &str) -> anyhow::Result<()> {
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    Ok(())
}

#[inline(always)]
pub async fn send_interative(
    bot: &Bot,
    dialogue: &BotDialogue,
    text: &str,
    buttons: &[(&str, (usize, &str))],
) -> anyhow::Result<Message> {
    let message = bot
        .send_message(dialogue.chat_id(), text)
        .reply_markup(buttons_to_inline_keyboard(buttons))
        .await?;

    Ok(message)
}

#[inline(always)]
pub async fn edit_interative(
    bot: &Bot,
    message: &Message,
    text: &str,
    buttons: &[(&str, (usize, &str))],
) -> anyhow::Result<Message> {
    let message = bot
        .edit_message_text(message.chat.id, message.id, text)
        .reply_markup(buttons_to_inline_keyboard(buttons))
        .await?;

    Ok(message)
}
