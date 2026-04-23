use std::env;
use std::fs;
use std::collections::HashMap;
use dotenv::dotenv;
use serde::Deserialize;

pub type Prompts = HashMap<String, String>;

pub type Messages = HashMap<String, String>;
pub type Buttons = HashMap<String, String>;
pub type Callbacks = HashMap<String, String>;

pub struct Locale {
    pub messages: Messages,
    pub buttons: Buttons,
    pub callbacks: Callbacks,
}

pub struct AppConfig {
    pub tg_bot_key: String,
    pub open_ai_key: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub model_name: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenv().ok();
        Self {
            tg_bot_key: env::var("TG_BOT_KEY").expect("TG_BOT_KEY missing"),
            open_ai_key: env::var("OPEN_AI_KEY").expect("OPEN_AI_KEY missing"),
            temperature: env::var("TEMPERATURE").unwrap_or_else(|_| "0.7".into()).parse().unwrap_or(0.7),
            max_tokens: env::var("MAX_TOKENS").unwrap_or_else(|_| "500".into()).parse().unwrap_or(500),
            model_name: env::var("MODEL_NAME").unwrap_or_else(|_| "gpt-4o-mini".into()),
        }
    }
}

pub fn load_prompts() -> Prompts {
    load_json("resources/prompts.json")
}

pub fn load_locale() -> Locale {
    Locale {
        messages: load_json("resources/locale/messages.json"),
        buttons: load_json("resources/locale/buttons.json"),
        callbacks: load_json("resources/locale/callbacks.json"),
    }
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &str) -> T {
    let data = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Unable to read {}", path));
    serde_json::from_str(&data)
        .unwrap_or_else(|_| panic!("Invalid JSON in {}", path))
}