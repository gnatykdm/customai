use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use dashmap::DashMap;
use crate::config::AppConfig;

pub struct OpenAIClient {
    pub client: Client<OpenAIConfig>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub history: DashMap<i64, Vec<ChatCompletionRequestMessage>>,
}

impl OpenAIClient {
    pub fn new(cfg: &AppConfig) -> Self {
        let openai_config = OpenAIConfig::new()
            .with_api_key(&cfg.open_ai_key);

        Self {
            client: Client::with_config(openai_config),
            model: cfg.model_name.clone(),
            temperature: cfg.temperature,
            max_tokens: cfg.max_tokens,
            history: DashMap::new(),
        }
    }

    pub fn clear_history(&self, chat_id: i64) {
        self.history.remove(&chat_id);
    }

    pub async fn ask(
        &self,
        chat_id: i64,
        system_instructions: &str,
        user_input: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

        let user_message: ChatCompletionRequestMessage =
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_input)
                .build()?
                .into();

        self.history
            .entry(chat_id)
            .or_default()
            .push(user_message);

        let system_message: ChatCompletionRequestMessage =
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_instructions)
                .build()?
                .into();

        let mut messages = vec![system_message];
        messages.extend(
            self.history
                .get(&chat_id)
                .map(|h| h.value().clone())
                .unwrap_or_default()
        );

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(self.max_tokens)
            .model(&self.model)
            .temperature(self.temperature)
            .messages(messages)
            .build()?;

        let response = self.client.chat().create(request).await?;

        let text = response.choices.get(0)
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "AI returned empty response".to_string());

        let assistant_message: ChatCompletionRequestMessage =
            ChatCompletionRequestAssistantMessageArgs::default()
                .content(text.clone())
                .build()?
                .into();

        self.history
            .entry(chat_id)
            .or_default()
            .push(assistant_message);

        Ok(text)
    }
}