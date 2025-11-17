#![allow(unused)]

use crate::bot::BotDialogue;
use std::borrow::Cow;
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup}};

#[inline(always)]
pub async fn send_msg(bot: &Bot, chat_id: ChatId, text: Cow<'_, str>) -> anyhow::Result<()> {
    bot.send_message(chat_id, text).await?;
    Ok(())
}

#[inline(always)]
pub async fn edit_msg(bot: &Bot, msg: &Message, text: Cow<'_, str>) -> anyhow::Result<()> {
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    Ok(())
}

#[inline(always)]
pub async fn send_interative(
    bot: &Bot,
    dialogue: &BotDialogue,
    text: Cow<'_, str>,
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
    text: Cow<'_, str>,
    buttons: &[(&str, (usize, &str))],
) -> anyhow::Result<Message> {
    let message = bot
        .edit_message_text(message.chat.id, message.id, text)
        .reply_markup(buttons_to_inline_keyboard(buttons))
        .await?;

    Ok(message)
}

pub fn buttons_to_inline_keyboard(buttons: &[(&str, (usize, &str))]) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new((0..buttons.len()).map(|idx| {
        buttons
            .iter()
            .filter(move |(_, (row, _))| *row == idx)
            .map(|(id, (_, text_key))| {
                let text = t!(*text_key);
                InlineKeyboardButton::callback(text, *id)
            })
    }))
}
