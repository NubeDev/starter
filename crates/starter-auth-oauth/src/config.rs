//! Configuration loader.
//!
//! Reads four kinds of values:
//!
//! - **Hostwide**: `OAUTH_BASE_URL`, `OAUTH_STATE_STORE`,
//!   `OAUTH_SIGNUP_ENABLED`, `OAUTH_SIGNUP_DEFAULT_ROLE`.
//! - **Per provider**: `OAUTH_<PROVIDER>_CLIENT_ID` and
//!   `OAUTH_<PROVIDER>_CLIENT_SECRET`. **Presence of both** enables
//!   the provider — there is no separate `enabled` flag (SCOPE
//!   Constraints). Absence of either drops the provider from the
//!   loaded set without an error.
//!
//! Each value resolves in this order:
//!
//! 1. The supplied `&dyn SecretStore`, when its `ready()` returns
//!    `true`. The starter-secrets-* impls land here.
//! 2. The process environment.
//! 3. Default (only the hostwide values have defaults; provider
//!    credentials never do).
//!
//! The secret-store lookup uses dotted names (`oauth.base_url`,
//! `oauth.github.client_secret`); the env-var fallback uses the
//! `OAUTH_*` shouty form. Both names are documented per field
//! below so an operator can pick either path without guessing.

use std::collections::BTreeMap;

use starter_spi::auth::Role;
use starter_spi::secrets::{Secret, SecretError, SecretStore};

/// Loaded OAuth configuration. Construct via [`OAuthConfig::load`].
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// External-facing base URL the operator advertises to the
    /// provider. The redirect URI for each provider is built as
    /// `{base_url}/auth/oauth/{provider}/callback`. No default —
    /// must be set.
    pub base_url: String,
    /// Where the in-flight [`crate::OAuthFlowState`] entries live.
    /// `Memory` is the default; `Sqlite` and `Postgres` land in
    /// Phase 4 and require their respective cargo features.
    pub state_store: StateStoreKind,
    /// `true` when a callback for an unknown user may create a
    /// fresh local account. Default `true`; `false` forces the
    /// callback to refuse first-time sign-ins with `HTTP 403`.
    pub signup_enabled: bool,
    /// Role assigned to a newly-created local user when no
    /// per-provider domain map produces a match.
    pub signup_default_role: Role,
    /// Enabled providers, keyed by provider id (`"github"`,
    /// `"google"`). Order is alphabetical (BTreeMap) so log lines
    /// listing the enabled set are stable.
    pub providers: BTreeMap<String, ProviderCredentials>,
}

/// Operator-supplied client credentials for one provider.
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    /// OAuth `client_id` shown on the provider's app dashboard.
    pub client_id: String,
    /// OAuth `client_secret`. Wrapped in [`Secret`] so a stray
    /// `Debug` cannot leak it.
    pub client_secret: Secret,
}

/// Which backend persists the short-lived `OAuthFlowState`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StateStoreKind {
    /// Single-process `HashMap` behind a `Mutex`. Default.
    #[default]
    Memory,
    /// `starter_auth_oauth_state` table in sqlite. Phase 4.
    Sqlite,
    /// Same table in postgres. Phase 4.
    Postgres,
}

impl StateStoreKind {
    fn parse(s: &str) -> Result<Self, OAuthConfigError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "memory" | "" => Ok(StateStoreKind::Memory),
            "sqlite" => Ok(StateStoreKind::Sqlite),
            "postgres" | "postgresql" => Ok(StateStoreKind::Postgres),
            other => Err(OAuthConfigError::Invalid(format!(
                "OAUTH_STATE_STORE: unknown value {other:?} (expected memory|sqlite|postgres)"
            ))),
        }
    }
}

/// Provider ids the loader knows about in v0.1. Adding a third
/// provider is one entry here plus one file in `providers/`.
const KNOWN_PROVIDERS: &[&str] = &["github", "google"];

/// Errors raised while loading configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OAuthConfigError {
    /// A required value was absent from both the secret store and
    /// the process environment.
    #[error("oauth config: missing required value {0}")]
    Missing(&'static str),
    /// A value was present but unparseable (bad bool, unknown
    /// state-store kind, etc.).
    #[error("oauth config: {0}")]
    Invalid(String),
    /// The secret store backend failed.
    #[error("oauth config: secret store: {0}")]
    Secret(#[from] SecretError),
}

impl OAuthConfig {
    /// Resolve configuration from the supplied secret store
    /// (preferred) with env-var fallback. Passing `None` for the
    /// store reads env-vars only — useful in tests and for
    /// consumers who have not wired `starter-secrets-*`.
    pub fn load(secrets: Option<&dyn SecretStore>) -> Result<Self, OAuthConfigError> {
        let base_url = required(secrets, "oauth.base_url", "OAUTH_BASE_URL")?;
        let state_store = optional(secrets, "oauth.state_store", "OAUTH_STATE_STORE")?
            .map(|v| StateStoreKind::parse(&v))
            .transpose()?
            .unwrap_or_default();
        let signup_enabled = optional(secrets, "oauth.signup_enabled", "OAUTH_SIGNUP_ENABLED")?
            .as_deref()
            .map(parse_bool)
            .transpose()?
            .unwrap_or(true);
        let signup_default_role = optional(
            secrets,
            "oauth.signup_default_role",
            "OAUTH_SIGNUP_DEFAULT_ROLE",
        )?
        .as_deref()
        .map(parse_role)
        .transpose()?
        .unwrap_or(Role::Reader);

        let mut providers = BTreeMap::new();
        for &id in KNOWN_PROVIDERS {
            if let Some(creds) = load_provider(secrets, id)? {
                providers.insert(id.to_string(), creds);
            }
        }

        Ok(Self {
            base_url,
            state_store,
            signup_enabled,
            signup_default_role,
            providers,
        })
    }

    /// `true` when `provider_id` was enabled (client_id +
    /// client_secret both resolved). Equivalent to
    /// `self.providers.contains_key(provider_id)` but reads better
    /// at call sites.
    pub fn provider_enabled(&self, provider_id: &str) -> bool {
        self.providers.contains_key(provider_id)
    }
}

fn load_provider(
    secrets: Option<&dyn SecretStore>,
    id: &str,
) -> Result<Option<ProviderCredentials>, OAuthConfigError> {
    let upper = id.to_ascii_uppercase();
    let id_name = format!("oauth.{id}.client_id");
    let secret_name = format!("oauth.{id}.client_secret");
    let id_env = format!("OAUTH_{upper}_CLIENT_ID");
    let secret_env = format!("OAUTH_{upper}_CLIENT_SECRET");

    let client_id = optional_dynamic(secrets, &id_name, &id_env)?;
    let client_secret = optional_dynamic(secrets, &secret_name, &secret_env)?;

    // SCOPE Constraints: presence of *both* enables the provider.
    // Half-configured (only client_id) almost certainly means the
    // operator started setting it up and stopped halfway; warn so
    // they notice, then drop the provider silently.
    match (client_id, client_secret) {
        (Some(client_id), Some(secret)) => Ok(Some(ProviderCredentials {
            client_id,
            client_secret: Secret::new(secret),
        })),
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            tracing::warn!(
                provider = id,
                "oauth provider configuration partial; provider disabled (both client_id and client_secret required)"
            );
            Ok(None)
        }
    }
}

fn required(
    secrets: Option<&dyn SecretStore>,
    secret_name: &str,
    env_name: &'static str,
) -> Result<String, OAuthConfigError> {
    optional(secrets, secret_name, env_name)?.ok_or(OAuthConfigError::Missing(env_name))
}

fn optional(
    secrets: Option<&dyn SecretStore>,
    secret_name: &str,
    env_name: &str,
) -> Result<Option<String>, OAuthConfigError> {
    optional_dynamic(secrets, secret_name, env_name)
}

fn optional_dynamic(
    secrets: Option<&dyn SecretStore>,
    secret_name: &str,
    env_name: &str,
) -> Result<Option<String>, OAuthConfigError> {
    if let Some(store) = secrets {
        if store.ready() {
            if let Some(value) = store.get(secret_name)? {
                return Ok(Some(value.into_inner()));
            }
        }
    }
    match std::env::var(env_name) {
        Ok(v) if !v.is_empty() => Ok(Some(v)),
        _ => Ok(None),
    }
}

fn parse_bool(s: &str) -> Result<bool, OAuthConfigError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(OAuthConfigError::Invalid(format!(
            "expected boolean, got {other:?}"
        ))),
    }
}

fn parse_role(s: &str) -> Result<Role, OAuthConfigError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "reader" => Ok(Role::Reader),
        "writer" => Ok(Role::Writer),
        "admin" => Ok(Role::Admin),
        other => Err(OAuthConfigError::Invalid(format!(
            "OAUTH_SIGNUP_DEFAULT_ROLE: unknown role {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use `set_var` on a mutex-guarded helper so the env mutation
    /// tests can't race each other. `cargo test` runs tests in one
    /// process by default, and the env is global state.
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear(name: &str) {
        std::env::remove_var(name);
    }

    #[test]
    fn missing_base_url_is_a_load_error() {
        let _g = ENV_LOCK.lock().unwrap();
        clear("OAUTH_BASE_URL");
        let err = OAuthConfig::load(None).unwrap_err();
        assert!(matches!(err, OAuthConfigError::Missing("OAUTH_BASE_URL")));
    }

    #[test]
    fn presence_of_both_credentials_enables_provider() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("OAUTH_BASE_URL", "https://app.example.com");
        std::env::set_var("OAUTH_GITHUB_CLIENT_ID", "id-123");
        std::env::set_var("OAUTH_GITHUB_CLIENT_SECRET", "sec-456");
        clear("OAUTH_GOOGLE_CLIENT_ID");
        clear("OAUTH_GOOGLE_CLIENT_SECRET");
        clear("OAUTH_SIGNUP_ENABLED");
        clear("OAUTH_SIGNUP_DEFAULT_ROLE");
        clear("OAUTH_STATE_STORE");

        let cfg = OAuthConfig::load(None).expect("load");
        assert!(cfg.provider_enabled("github"));
        assert!(!cfg.provider_enabled("google"));
        assert_eq!(cfg.state_store, StateStoreKind::Memory);
        assert!(cfg.signup_enabled);
        assert_eq!(cfg.signup_default_role, Role::Reader);

        clear("OAUTH_GITHUB_CLIENT_ID");
        clear("OAUTH_GITHUB_CLIENT_SECRET");
        clear("OAUTH_BASE_URL");
    }

    #[test]
    fn partial_credentials_disables_provider() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("OAUTH_BASE_URL", "https://app.example.com");
        std::env::set_var("OAUTH_GITHUB_CLIENT_ID", "id-123");
        clear("OAUTH_GITHUB_CLIENT_SECRET");
        clear("OAUTH_GOOGLE_CLIENT_ID");
        clear("OAUTH_GOOGLE_CLIENT_SECRET");

        let cfg = OAuthConfig::load(None).expect("load");
        assert!(!cfg.provider_enabled("github"));

        clear("OAUTH_GITHUB_CLIENT_ID");
        clear("OAUTH_BASE_URL");
    }
}
