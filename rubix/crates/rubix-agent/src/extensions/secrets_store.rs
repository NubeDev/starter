//! Rubix-side [`SecretStore`] resolution at boot.
//!
//! The host has two backings to choose from:
//!
//! 1. [`starter_secrets_keyring::KeyringSecretStore`] — OS keyring.
//!    Works on a developer's workstation (login keyring on Linux,
//!    Keychain on macOS, Credential Manager on Windows). The
//!    keyring crate returns `false` from `ready()` when the
//!    platform service isn't available (no DBus in CI, headless
//!    Linux container, etc.).
//! 2. [`EnvSecretStore`] — env-var fallback. Reads
//!    `RUBIX_SECRET_<NAME>` for each `get(name)`. Names are
//!    upper-cased and non-alphanumerics turned into `_` so an
//!    extension asking for `"ai:anthropic:api_key"` finds it
//!    under `RUBIX_SECRET_AI_ANTHROPIC_API_KEY`.
//!
//! [`pick_default`] returns the keyring store when it's ready,
//! else the env-var store. Operators who need a different backing
//! (HashiCorp Vault, age-encrypted file) can swap in their own
//! [`SecretStore`] impl — the extension host doesn't care which
//! one it gets.

use std::sync::Arc;

use starter_spi::secrets::{Secret, SecretError, SecretStore};

/// Build the rubix-default `SecretStore`. Returns the keyring
/// backing when the platform service answers a probe; falls back
/// to [`EnvSecretStore`] otherwise.
///
/// `binary` is forwarded to the keyring backend as the keyring
/// service name (so two starter-based apps don't collide).
pub fn pick_default(binary: &str) -> Arc<dyn SecretStore> {
    let keyring = starter_secrets_keyring::KeyringSecretStore::new(binary);
    if keyring.ready() {
        return Arc::new(keyring);
    }
    Arc::new(EnvSecretStore::new())
}

/// Env-var-backed `SecretStore`. Looks up
/// `RUBIX_SECRET_<UPPERCASED_NAME>` for each `get`. `put` and
/// `delete` are no-ops that return `Ok(())` — the env can't be
/// mutated from inside the process in a way that survives, and
/// extensions don't need that surface in v0.1.
#[derive(Debug, Default)]
pub struct EnvSecretStore;

impl EnvSecretStore {
    /// Construct.
    pub fn new() -> Self {
        Self
    }

    /// Map a logical secret name to its env-var key. Non-alphanumerics
    /// become `_`; ASCII letters are upper-cased.
    fn env_key(name: &str) -> String {
        let mut out = String::with_capacity("RUBIX_SECRET_".len() + name.len());
        out.push_str("RUBIX_SECRET_");
        for c in name.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_uppercase());
            } else {
                out.push('_');
            }
        }
        out
    }
}

impl SecretStore for EnvSecretStore {
    fn ready(&self) -> bool {
        // Env vars are always available; readiness is a no-op.
        true
    }

    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
        let key = Self::env_key(name);
        match std::env::var(&key) {
            Ok(value) => Ok(Some(Secret::new(value))),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(SecretError::Backend(format!(
                "env var {key} is not valid UTF-8"
            ))),
        }
    }

    fn put(&self, _name: &str, _value: Secret) -> Result<(), SecretError> {
        // No-op: env mutations don't survive process restarts and
        // would surprise operators who set the var on the command
        // line. Extensions that need to *write* secrets should
        // not be using the env-var fallback.
        Ok(())
    }

    fn delete(&self, _name: &str) -> Result<(), SecretError> {
        // Same posture as `put`.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_key_uppercases_and_normalises() {
        assert_eq!(
            EnvSecretStore::env_key("ai:anthropic:api_key"),
            "RUBIX_SECRET_AI_ANTHROPIC_API_KEY"
        );
        assert_eq!(
            EnvSecretStore::env_key("simple.dotted"),
            "RUBIX_SECRET_SIMPLE_DOTTED"
        );
    }

    #[test]
    fn env_get_resolves_set_var_and_misses_unset() {
        // Use a name that's very unlikely to be set externally.
        let name = "starter_tests_envsecret_xyzzy";
        let key = EnvSecretStore::env_key(name);
        // Sanity: ensure clean state.
        // SAFETY: env mutation in tests is process-global; this
        // test names a unique var so concurrent tests don't
        // observe it.
        // SAFETY: env mutation is process-global; we choose a
        // unique var name above so it doesn't leak into other tests.
        unsafe {
            std::env::remove_var(&key);
        }
        let store = EnvSecretStore::new();
        assert!(store.get(name).unwrap().is_none(), "missing var → None");
        unsafe {
            std::env::set_var(&key, "hunter2");
        }
        let got = store.get(name).unwrap().expect("set");
        assert_eq!(got.expose(), "hunter2");
        unsafe {
            std::env::remove_var(&key);
        }
    }

    #[test]
    fn ready_is_always_true() {
        assert!(EnvSecretStore::new().ready());
    }
}
