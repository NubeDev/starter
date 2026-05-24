//! Layered agent configuration.
//!
//! Replaces the ad-hoc `std::env::var(...)` calls in `main.rs` with
//! a single [`starter_config::Loader`] pipeline:
//!
//!   defaults  <  `$XDG_CONFIG_HOME/rubix/agent.toml`  <  `RUBIX_*`
//!
//! The struct is intentionally small — it carries only the wiring
//! knobs the binary itself needs at boot. Domain knobs (e.g. the
//! disk tool's history host id) stay on their own types per the
//! verb-per-file rule. See
//! [docs/design/config/](../../../docs/design/config/README.md).
//!
//! All fields are optional inside the loader. Unset values fall
//! through to the per-field defaults below so a developer can boot
//! the agent with `cargo run -p rubix-agent` against no config file
//! and no env vars at all (Postgres + ClickHouse are then skipped
//! by the migration steps as documented in their own boot files).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use starter_config::Loader;

/// What `main.rs` reads at boot. The fields cover the four wiring
/// inputs the binary needs and nothing more.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Bind address for the HTTP listener. Mirrors the legacy
    /// `RUBIX_BIND` env var. Default `127.0.0.1:8088`.
    pub bind: String,

    /// Postgres DSN (changelog + auth-users tables + authz). When
    /// `None` the binary boots without DB-backed features; see
    /// [`crate::boot::migrations`].
    pub database_url: Option<String>,

    /// ClickHouse HTTP endpoint (history + warehouse). When
    /// `None` the binary boots without the warehouse; see
    /// [`crate::boot::clickhouse`].
    pub clickhouse_url: Option<String>,

    /// Path to the on-disk secrets directory. Reserved for the
    /// upcoming JWT signing key / OAuth client secret material.
    pub secrets_path: Option<PathBuf>,

    /// Explicit path to the config file. When `None` the loader
    /// falls back to `$XDG_CONFIG_HOME/rubix/agent.toml` then
    /// `$HOME/.config/rubix/agent.toml`.
    pub config_path: Option<PathBuf>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8088".to_owned(),
            database_url: None,
            clickhouse_url: None,
            secrets_path: None,
            config_path: None,
        }
    }
}

impl AgentConfig {
    /// Compose the loader chain and return the resolved config.
    ///
    /// Order (later wins):
    ///   1. [`AgentConfig::default`]
    ///   2. TOML file at the resolved [`Self::default_config_path`]
    ///   3. Env vars prefixed `RUBIX_` (double-underscore = nested)
    pub fn load() -> Result<Self, starter_config::ConfigError> {
        let cfg_path = std::env::var_os("RUBIX_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(Self::default_config_path);
        let mut loaded: Self = Loader::with_defaults(Self::default())
            .with_file(cfg_path.to_string_lossy().into_owned())
            .with_env("RUBIX_")
            .load()?;

        // The two legacy env names predate the loader and stay
        // honored verbatim so existing deployments keep working
        // without editing systemd unit files. The loader's
        // `RUBIX_DATABASE_URL` / `RUBIX_CLICKHOUSE_URL` paths win
        // when both are set.
        if loaded.database_url.is_none() {
            loaded.database_url = std::env::var("RUBIX_DSN").ok();
        }
        if loaded.clickhouse_url.is_none() {
            loaded.clickhouse_url = std::env::var("RUBIX_CH_URL").ok();
        }
        if loaded.config_path.is_none() {
            loaded.config_path = Some(cfg_path);
        }
        Ok(loaded)
    }

    /// `$XDG_CONFIG_HOME/rubix/agent.toml`, falling back to
    /// `$HOME/.config/rubix/agent.toml`. Missing parents are not an
    /// error — [`Loader::with_file`] silently skips an absent file.
    pub fn default_config_path() -> PathBuf {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("rubix/agent.toml");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".config/rubix/agent.toml");
        }
        PathBuf::from("agent.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bind_is_localhost_8088() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.bind, "127.0.0.1:8088");
        assert!(cfg.database_url.is_none());
        assert!(cfg.clickhouse_url.is_none());
    }

    #[test]
    fn default_config_path_uses_xdg_when_set() {
        let prior = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg");
        let p = AgentConfig::default_config_path();
        assert!(p.ends_with("rubix/agent.toml"));
        assert!(p.to_string_lossy().contains("/tmp/xdg"));
        match prior {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}
