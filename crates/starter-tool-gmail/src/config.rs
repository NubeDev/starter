//! [`GmailConfig`] — already-resolved credentials handed to the tool.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, the future
//! `starter-auth-oauth`, or a custom flow) and constructs this
//! struct.

use starter_spi::SecretString;

/// Default Gmail REST API host. Tests override via
/// [`GmailConfig::base_url`]; production callers pass
/// [`GmailConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://gmail.googleapis.com";

/// Default `user_id` path segment on
/// `users/{userId}/messages/send`. `"me"` resolves to the mailbox
/// owning the access token and is the right default for both
/// personal tokens and per-user OAuth grants.
const DEFAULT_USER_ID: &str = "me";

/// Resolved credentials + endpoint for the Gmail REST API.
///
/// Token acquisition is **not** this crate's job. The consumer
/// composes a flow (interactive consent via the future
/// `starter-auth-oauth`, a long-lived refresh token, a
/// service-account exchange, …) and hands the resulting bearer token
/// in here. Token refresh on 401 is also the consumer's
/// responsibility today — the tool surfaces a 401 as
/// [`starter_spi::Error::Unauthenticated`] so a wrapper can trigger
/// it without inspecting the source chain.
pub struct GmailConfig {
    /// OAuth2 access token (the literal string passed to
    /// `Authorization: Bearer …`). A `SecretString` so it stays out
    /// of `Debug` and `Display` output by default.
    pub oauth_access_token: SecretString,
    /// The Gmail user id on the path. `"me"` for the
    /// caller-as-mailbox case; an explicit address only when domain
    /// delegation is in play.
    pub user_id: String,
    /// Base URL of the Gmail REST API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at
    /// a mock server.
    pub base_url: String,
}

impl GmailConfig {
    /// The production Gmail API base URL
    /// (`https://gmail.googleapis.com`). Exposed as a function rather
    /// than a `pub const` so the type stays a plain owned `String`
    /// everywhere.
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }

    /// The default `user_id` (`"me"`). See the field doc — `"me"`
    /// matches the authenticated user and is the right default for
    /// 99% of consumers.
    pub fn default_user_id() -> String {
        DEFAULT_USER_ID.to_string()
    }
}
