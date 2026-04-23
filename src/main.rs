mod config;
mod client;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode};
use config::{AppConfig, load_prompts, load_locale};
use crate::client::openai::OpenAIClient;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone)]
struct BotState {
    ai_client:    Arc<OpenAIClient>,
    prompts:      Arc<config::Prompts>,
    locale:       Arc<config::Locale>,
    bot_messages: Arc<Mutex<HashMap<i64, Vec<MessageId>>>>,
}

impl BotState {
    fn msg(&self, key: &str) -> &str {
        self.locale.messages.get(key).map(|s| s.as_str()).unwrap_or("")
    }

    fn prompt(&self, key: &str) -> Option<&str> {
        self.prompts.get(key).map(|s| s.as_str())
    }

    async fn track(&self, chat_id: ChatId, msg_id: MessageId) {
        self.bot_messages
            .lock()
            .await
            .entry(chat_id.0)
            .or_default()
            .push(msg_id);
    }

    async fn take_messages(&self, chat_id: ChatId) -> Vec<MessageId> {
        self.bot_messages
            .lock()
            .await
            .remove(&chat_id.0)
            .unwrap_or_default()
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Bot is starting...");

    let cfg     = Arc::new(AppConfig::from_env());
    let prompts = Arc::new(load_prompts());
    let locale  = Arc::new(load_locale());
    let ai_client = Arc::new(OpenAIClient::new(&cfg));
    let bot     = Bot::new(&cfg.tg_bot_key);

    let state = BotState {
        ai_client,
        prompts,
        locale,
        bot_messages: Arc::new(Mutex::new(HashMap::new())),
    };

    log::info!("Connected to Telegram. Listening for messages...");

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn build_keyboard(state: &BotState) -> InlineKeyboardMarkup {
    let mut valid_buttons: Vec<InlineKeyboardButton> = state.locale.buttons
        .iter()
        .filter_map(|(btn_key, btn_text)| {
            if btn_text.is_empty() { return None; }
            let cb_key  = btn_key.replace("_button", "_callback");
            let cb_data = state.locale.callbacks.get(&cb_key)?;
            if cb_data.is_empty() { return None; }
            Some(InlineKeyboardButton::callback(btn_text, cb_data))
        })
        .collect();

    valid_buttons.sort_by(|a, b| a.text.cmp(&b.text));

    let rows: Vec<Vec<InlineKeyboardButton>> = valid_buttons
        .chunks(2)
        .map(|chunk| chunk.to_vec())
        .collect();

    InlineKeyboardMarkup::new(rows)
}

fn sanitize_html(text: &str) -> String {
    let mut s = text.to_string();

    // **bold** → <b>bold</b>
    while s.contains("**") {
        s = s.replacen("**", "<b>", 1).replacen("**", "</b>", 1);
    }

    s.replace("<ul>", "")
        .replace("</ul>", "\n")
        .replace("<ol>", "")
        .replace("</ol>", "\n")
        .replace("<li>", "• ")
        .replace("</li>", "\n")
        .replace("<h1>", "<b>").replace("</h1>", "</b>\n")
        .replace("<h2>", "<b>").replace("</h2>", "</b>\n")
        .replace("<h3>", "<b>").replace("</h3>", "</b>\n")
        .replace("<p>", "").replace("</p>", "\n")
        .replace("<br>", "\n").replace("<br/>", "\n")
        .replace("<strong>", "<b>").replace("</strong>", "</b>")
        .replace("<em>", "<i>").replace("</em>", "</i>")
        .replace("<div>", "").replace("</div>", "\n")
        .replace("<span>", "").replace("</span>", "")
        .replace("\n\n\n", "\n\n")
        .trim()
        .to_string()
}

async fn send_ai_response(
    bot:          &Bot,
    chat_id:      ChatId,
    state:        &BotState,
    system_prompt: &str,
    user_input:   &str,
) -> HandlerResult {
    log::info!("AI request | chat_id={}", chat_id);
    let _ = bot.send_chat_action(chat_id, teloxide::types::ChatAction::Typing).await;

    match state.ai_client.ask(chat_id.0, system_prompt, user_input).await {
        Ok(response) => {
            log::info!("AI response | chat_id={} | length={} chars", chat_id, response.len());
            let clean = sanitize_html(&response);
            let sent = bot.send_message(chat_id, clean)
                .parse_mode(ParseMode::Html)
                .await?;
            state.track(chat_id, sent.id).await;
        }
        Err(e) => {
            log::error!("AI error | chat_id={} | error={}", chat_id, e);
            let sent = bot.send_message(chat_id, state.msg("error_message"))
                .parse_mode(ParseMode::Html)
                .await?;
            state.track(chat_id, sent.id).await;
        }
    }

    Ok(())
}

async fn delete_all_bot_messages(bot: &Bot, chat_id: ChatId, state: &BotState) {
    let ids = state.take_messages(chat_id).await;
    log::info!("Deleting {} bot messages | chat_id={}", ids.len(), chat_id);

    for msg_id in ids {
        if let Err(e) = bot.delete_message(chat_id, msg_id).await {
            log::warn!("Could not delete message {} | chat_id={} | {}", msg_id.0, chat_id, e);
        }
    }
}

async fn message_handler(bot: Bot, msg: Message, state: BotState) -> HandlerResult {
    let Some(text) = msg.text() else { return Ok(()); };
    let chat_id = msg.chat.id;

    let username = msg.from()
        .map(|u| u.username.as_deref().unwrap_or(u.first_name.as_str()))
        .unwrap_or("Unknown User");

    log::info!("Message | chat_id={} | user=@{}", chat_id, username);

    if let Some(cmd) = text.strip_prefix('/') {
        let msg_key = format!("{}_message", cmd);

        match cmd {
            "reset" => {
                log::info!("Command /reset | chat_id={}", chat_id);
                state.ai_client.clear_history(chat_id.0);
                delete_all_bot_messages(&bot, chat_id, &state).await;

                let sent = bot.send_message(chat_id, state.msg("reset_message"))
                    .parse_mode(ParseMode::Html)
                    .await?;
                state.track(chat_id, sent.id).await;
                return Ok(());
            }

            "start" | "help" => {
                log::info!("Command /{} | chat_id={}", cmd, chat_id);
                let message_text = state.msg(&msg_key);
                let text_to_send = if message_text.is_empty() {
                    state.msg("welcome_message")
                } else {
                    message_text
                };
                let sent = bot.send_message(chat_id, text_to_send)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(build_keyboard(&state))
                    .await?;
                state.track(chat_id, sent.id).await;
                return Ok(());
            }

            _ => {
                log::info!("Command /{} | chat_id={}", cmd, chat_id);

                let msg_text = state.msg(&msg_key);
                if !msg_text.is_empty() {
                    let sent = bot.send_message(chat_id, msg_text)
                        .parse_mode(ParseMode::Html)
                        .await?;
                    state.track(chat_id, sent.id).await;
                }

                if text.starts_with('/') {
                    return Ok(());
                }

                let prompt_key = format!("{}_prompt", cmd);
                if let Some(system_prompt) = state.prompt(&prompt_key) {
                    send_ai_response(&bot, chat_id, &state, system_prompt, text).await?;
                }

                return Ok(());
            }
        }
    }

    if text.starts_with('/') {
        return Ok(());
    }

    let system_prompt = state.prompt("assistant_system_prompt").unwrap_or("");
    send_ai_response(&bot, chat_id, &state, system_prompt, text).await?;

    Ok(())
}

async fn callback_handler(bot: Bot, q: CallbackQuery, state: BotState) -> HandlerResult {
    let Some(data) = q.data.as_deref() else {
        bot.answer_callback_query(&q.id).await?;
        return Ok(());
    };

    bot.answer_callback_query(&q.id).await?;

    let Some(chat_id) = q.message.as_ref().map(|m| m.chat.id) else {
        return Ok(());
    };

    let username = q.from.username.as_deref().unwrap_or("unknown");
    log::info!("Callback | chat_id={} | user={} | data=\"{}\"", chat_id, username, data);

    let action_key = state.locale.callbacks
        .iter()
        .find(|(_, cb_value)| cb_value.as_str() == data)
        .map(|(cb_key, _)| cb_key.replace("_callback", ""));

    let Some(action_key) = action_key else {
        log::warn!("Callback not found in callbacks.json | data=\"{}\"", data);
        return Ok(());
    };

    log::info!("Callback resolved | action_key=\"{}\"", action_key);

    if action_key == "reset" {
        state.ai_client.clear_history(chat_id.0);
        log::info!("History cleared in AI client | chat_id={}", chat_id);

        if let Some(ref m) = q.message {
            let _ = bot.delete_message(chat_id, m.id).await;
        }

        delete_all_bot_messages(&bot, chat_id, &state).await;

        let sent = bot.send_message(chat_id, state.msg("reset_message"))
            .parse_mode(ParseMode::Html)
            .await?;
        state.track(chat_id, sent.id).await;

        return Ok(());
    }

    let msg_key  = format!("{}_message", action_key);
    let msg_text = state.msg(&msg_key);

    if !msg_text.is_empty() {
        log::info!("Sending static message | key=\"{}\"", msg_key);
        let sent = bot.send_message(chat_id, msg_text)
            .parse_mode(ParseMode::Html)
            .await?;
        state.track(chat_id, sent.id).await;
    } else {
        log::warn!("No message found | key=\"{}\"", msg_key);
    }

    Ok(())
}