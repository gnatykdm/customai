use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Prompts {
    pub assistant_system_prompt: String,
    pub summarizer_prompt: String,
}

#[derive(Deserialize)]
pub struct Locale {
    pub welcome_message: String,
    pub help_button: String,
    pub settings_button: String,
}

pub fn load_prompts() -> Prompts {
    let data = fs::read_to_string("resources/prompts.json").expect("Unable to read prompts.json");
    serde_json::from_str(&data).expect("JSON was not well-formatted")
}

pub fn load_locale() -> Locale {
    let data = fs::read_to_string("resources/locale.json").expect("Unable to read locale.json");
    serde_json::from_str(&data).expect("JSON was not well-formatted")
}