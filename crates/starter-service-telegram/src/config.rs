//! [`TelegramBotConfig`] — already-resolved credentials handed to
//! the inbound service.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, or a literal in
//! dev) and constructs this struct.

use starter_spi::SecretString;

/// Default Telegram Bot API host. Tests override via
/// [`TelegramBotConfig::base_url`]; production callers pass
/// [`TelegramBotConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://api.telegram.org";

/// Long-poll timeout sent to `getUpdates`. Telegram caps the upper
/// bound at 50s; 30s matches the value the codeless reference
/// commits to and keeps the worst-case shutdown latency bounded —
/// the loop watches `ctx.shutdown` in `tokio::select!` with the HTTP
/// future, so 30s is *also* the worst-case wait between a shutdown
/// signal and a clean exit on a chatty bot.
pub const LONG_POLL_TIMEOUT_SECS: u64 = 30;

/// Resolved credentials + endpoint for the Telegram Bot API.
///
/// Sibling field shape to [`starter_tool_telegram::TelegramConfig`] —
/// a consumer with both crates in scope can build both from the same
/// `bot_token` / `base_url` pair without copy-paste config code.
pub struct TelegramBotConfig {
    /// Bot token (`<bot_id>:<secret>`) issued by BotFather. Must
    /// carry the same permissions as the outbound crate's token (in
    /// the simple case it *is* the same token).
    pub bot_token: SecretString,
    /// Base URL of the Telegram Bot API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at
    /// a mock server.
    pub base_url: String,
}

impl TelegramBotConfig {
    /// The production Telegram Bot API base URL
    /// (`https://api.telegram.org`).
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }
}
