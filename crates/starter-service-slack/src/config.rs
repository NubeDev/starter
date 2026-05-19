//! [`SlackSocketModeConfig`] — already-resolved credentials handed to
//! the inbound service.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, or a literal in
//! dev) and constructs this struct.

use starter_spi::SecretString;

/// Default Slack Web API base URL. Tests override via
/// [`SlackSocketModeConfig::base_url`]; production callers pass
/// [`SlackSocketModeConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://slack.com/api";

/// Resolved credentials + endpoint for Slack Socket Mode.
///
/// `app_token` is the **app-level** `xapp-…` token that
/// `apps.connections.open` requires — distinct from the bot user OAuth
/// `xoxb-…` token `starter-tool-slack` uses. The two cannot be merged
/// because Slack scopes them differently: the app token cannot post a
/// message, the bot token cannot open a socket-mode connection.
pub struct SlackSocketModeConfig {
    /// App-level OAuth token (`xapp-…`). Must carry the
    /// `connections:write` scope.
    pub app_token: SecretString,
    /// Base URL of the Slack Web API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at a
    /// mock server.
    pub base_url: String,
}

impl SlackSocketModeConfig {
    /// The production Slack Web API base URL
    /// (`https://slack.com/api`).
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }
}
