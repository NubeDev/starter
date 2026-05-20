//! [`GitHubConfig`] — already-resolved credentials handed to the tool.
//!
//! SCOPE R5: the provider crate does **not** read env vars or files;
//! the consumer's `main.rs` resolves secrets (via
//! `starter-secrets-keyring`, `starter-secrets-file`, or a literal in
//! dev) and constructs this struct.

use starter_spi::SecretString;

/// Default GitHub REST API base URL. Tests override via
/// [`GitHubConfig::base_url`]; production callers pass
/// [`GitHubConfig::default_base_url`].
const DEFAULT_BASE_URL: &str = "https://api.github.com";

/// Resolved credentials + endpoint for the GitHub REST API.
///
/// `access_token` is a personal access token (`ghp_…`) or an OAuth
/// bearer token carrying the `repo` scope. The same token the
/// `starter-auth-oauth` GitHub provider would issue works here.
pub struct GitHubConfig {
    /// Personal access token or OAuth bearer. Must carry the `repo`
    /// scope for issue creation (or `public_repo` for public repos only).
    pub access_token: SecretString,
    /// Base URL of the GitHub REST API. Set to the result of
    /// [`Self::default_base_url`] in production; tests point this at
    /// a mock server.
    pub base_url: String,
}

impl GitHubConfig {
    /// The production GitHub REST API base URL
    /// (`https://api.github.com`). Exposed as a function rather than
    /// a `pub const` so the type stays a plain owned `String`
    /// everywhere.
    pub fn default_base_url() -> String {
        DEFAULT_BASE_URL.to_string()
    }
}
