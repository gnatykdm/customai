# customai

**customai** is a Rust-based Telegram bot starter kit that combines `teloxide` with OpenAI-powered AI responses. It is designed for fast customization using JSON-driven prompts, locale messages, buttons, and callback actions.

## 🚀 What it does

- Runs as a Telegram bot using `teloxide`
- Sends and receives messages in chat
- Uses OpenAI via `async-openai` for AI-assisted replies
- Stores per-chat conversation history for richer context
- Loads prompts and localized bot text from JSON files
- Supports interactive inline buttons and callback commands

## ⚙️ Key features

- JSON-configurable bot prompts in `resources/prompts.json`
- Localized message text, buttons, and callback actions under `resources/locale/`
- Environment-driven configuration for API keys and model settings
- Built-in commands like `/start`, `/help`, `/reset`
- Session-based chat history with reset support

## 📁 Project structure

- `src/main.rs` — bot entry point, message/callback handlers, keyboard generation
- `src/config.rs` — environment config loader and JSON resource loader
- `src/client/openai.rs` — OpenAI chat client with per-chat history support
- `resources/prompts.json` — system and command prompts for OpenAI
- `resources/locale/` — localized bot UI content and callback values

## 🧩 Environment setup

Create a `.env` file in the project root with:

```env
TG_BOT_KEY=your_telegram_bot_token
OPEN_AI_KEY=your_openai_api_key
TEMPERATURE=0.7
MAX_TOKENS=500
MODEL_NAME=gpt-4o-mini
```

> `TEMPERATURE`, `MAX_TOKENS`, and `MODEL_NAME` are optional. Defaults are `0.7`, `500`, and `gpt-4o-mini`.

## ▶️ Run locally

1. Install Rust toolchain if needed: `rustup toolchain install stable`
2. Build and run the bot:
   ```bash
   cargo run --release
   ```

3. Start your Telegram bot and send `/start` or `/help` to begin.

## ✨ Customize the bot

- Edit `resources/prompts.json` to change AI behavior and add new command prompts
- Update `resources/locale/messages.json` for welcome/help/error text
- Adjust `resources/locale/buttons.json` and `callbacks.json` to change button labels and callback actions
- Add new commands by extending `src/main.rs` and mapping prompts/messages by name

## 🧪 Notes

- The bot uses OpenAI chat completions and preserves conversation history per chat.
- Use `/reset` to clear the current chat history and start fresh.
- If a system prompt is defined for a command, the bot will forward the user input to OpenAI using that prompt.

## 📜 License

This project is available under the terms of the `LICENSE` file.
