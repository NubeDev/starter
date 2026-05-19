//! [`SlackConfig`] — already-resolved credentials handed to the tool.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, or a literal in
//! dev) and constructs this struct.

use starter_spi::SecretString;

/// Default Slack Web API base URL. Tests override via
/// [`SlackConfig::base_url`]; production callers pass
/// [`SlackConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://slack.com/api";

/// Resolved credentials + endpoint for the Slack Web API.
///
/// `bot_token` is the `xoxb-…` token used in the `Authorization`
/// header on `chat.postMessage` (and every other Web API call).
///
/// `signing_secret` is held here even though the outbound tool does
/// not use it — the inbound `starter-service-slack` (later stage)
/// verifies request signatures with it, and forcing both fields into
/// the same config struct keeps a consumer's `main.rs` from carrying
/// two parallel Slack config structs once the service side ships.
pub struct SlackConfig {
    /// Bot user OAuth token. Must carry the `chat:write` scope.
    pub bot_token: SecretString,
    /// App signing secret. Unused by [`crate::SlackPostTool`]; required
    /// by the inbound service side for HMAC verification.
    pub signing_secret: SecretString,
    /// Base URL of the Slack Web API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at
    /// a mock server.
    pub base_url: String,
}

impl SlackConfig {
    /// The production Slack Web API base URL
    /// (`https://slack.com/api`). Exposed as a function rather than a
    /// `pub const` so the type stays a plain owned `String` everywhere.
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }
}
