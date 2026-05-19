//! [`TelegramConfig`] — already-resolved credentials handed to the tool.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, or a literal in
//! dev) and constructs this struct.

use starter_spi::SecretString;

/// Default Telegram Bot API host. Tests override via
/// [`TelegramConfig::base_url`]; production callers pass
/// [`TelegramConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://api.telegram.org";

/// Resolved credentials + endpoint for the Telegram Bot API.
///
/// The Bot API URL shape is
/// `<base_url>/bot<bot_token>/<method>`; the token is part of the
/// URL path, not an `Authorization` header. The crate concatenates
/// them at call time so the token is never copied into a `format!`
/// argument the operator might accidentally log.
pub struct TelegramConfig {
    /// Bot token (`<bot_id>:<secret>`) issued by BotFather.
    pub bot_token: SecretString,
    /// Base URL of the Telegram Bot API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at
    /// a mock server.
    pub base_url: String,
}

impl TelegramConfig {
    /// The production Telegram Bot API base URL
    /// (`https://api.telegram.org`). Exposed as a function rather than
    /// a `pub const` so the type stays a plain owned `String`
    /// everywhere.
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }
}
