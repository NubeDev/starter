//! Secret-store resolution + DSN password splicing at boot.
//!
//! Two responsibilities, both keyed off [`AgentConfig`]:
//!
//! 1. [`build_secrets_store`] picks the backing
//!    [`SecretStore`](starter_spi::secrets::SecretStore) the host
//!    uses. When `secrets_path` is set it opens an age-encrypted
//!    [`FileSecretStore`](starter_secrets_file::FileSecretStore)
//!    rooted there (headless hosts with no OS keyring); otherwise it
//!    falls back to the existing keyring → env-var chain in
//!    [`crate::extensions::secrets_store::pick_default`].
//!
//! 2. [`resolve_database_url`] takes the plain DSN from config and,
//!    when `database_password_secret` is set, looks that secret up in
//!    the store and splices it into the DSN's password component. A
//!    password-less DSN in config plus a one-time
//!    `rubix-admin secrets set db:password <pwd>` keeps the password
//!    out of the config file while every downstream pool builder
//!    (`migrations`, `undo_sweep`, the MCP pool, …) keeps reading the
//!    single resolved `cfg.database_url`.
//!
//! The legacy `RUBIX_DSN` / `RUBIX_DATABASE_URL` path is untouched:
//! with `database_password_secret` unset, [`resolve_database_url`]
//! returns the DSN verbatim.

use std::sync::Arc;

use starter_spi::secrets::SecretStore;

use super::config::AgentConfig;
use crate::extensions::secrets_store::pick_default;

/// Keyring service / file-store binary name for the agent's secrets.
const SECRETS_BINARY: &str = "rubix-agent";

/// Failures while resolving the database password from the store.
#[derive(Debug, thiserror::Error)]
pub enum SecretsBootError {
    /// `database_password_secret` named a key the store doesn't hold.
    #[error("database password secret {name:?} not found in the secrets store")]
    SecretMissing {
        /// The configured secret name that was looked up.
        name: String,
    },

    /// The store backend errored while reading the secret.
    #[error("reading database password secret {name:?}: {source}")]
    StoreRead {
        /// The configured secret name that was looked up.
        name: String,
        /// Wrapped lower-level error.
        #[source]
        source: starter_spi::secrets::SecretError,
    },

    /// `database_password_secret` was set but `database_url` was not,
    /// so there is no DSN to splice the password into.
    #[error(
        "database_password_secret is set but database_url is unset — \
         nothing to splice the password into"
    )]
    NoDatabaseUrl,

    /// `database_url` did not parse as a URL.
    #[error("database_url {dsn:?} is not a valid URL: {source}")]
    BadDsn {
        /// The DSN that failed to parse.
        dsn: String,
        /// Wrapped parse error.
        #[source]
        source: url::ParseError,
    },

    /// `url` rejected setting the password (e.g. the DSN is a
    /// cannot-be-a-base URL with no authority section).
    #[error("could not set password on database_url {dsn:?}")]
    SetPassword {
        /// The DSN that rejected a password component.
        dsn: String,
    },
}

/// Build the host `SecretStore`.
///
/// * `cfg.secrets_path` set → an age-encrypted
///   [`FileSecretStore`](starter_secrets_file::FileSecretStore)
///   rooted at that directory.
/// * otherwise → [`pick_default`] (keyring when ready, else env-var).
///
/// On a `FileSecretStore` build failure we log and fall back to the
/// default chain rather than aborting boot — a misconfigured path
/// shouldn't take the agent down when the keyring/env path still
/// works.
pub fn build_secrets_store(cfg: &AgentConfig) -> Arc<dyn SecretStore> {
    let Some(path) = cfg.secrets_path.as_ref() else {
        return pick_default(SECRETS_BINARY);
    };

    match starter_secrets_file::FileSecretStoreBuilder::new(SECRETS_BINARY)
        .data_dir(path.clone())
        .build()
    {
        Ok(store) => Arc::new(store),
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %path.display(),
                "secrets_path set but the file secret store failed to open; \
                 falling back to keyring/env-var chain"
            );
            pick_default(SECRETS_BINARY)
        }
    }
}

/// Resolve the effective Postgres DSN, splicing in the password from
/// the secrets store when `database_password_secret` is configured.
///
/// Returns `cfg.database_url` unchanged when the knob is unset (the
/// `RUBIX_DSN` path). When set, looks the secret up in `store` and
/// replaces the DSN's password component, returning a hard error if
/// the secret is missing rather than connecting with no password.
pub fn resolve_database_url(
    cfg: &AgentConfig,
    store: &dyn SecretStore,
) -> Result<Option<String>, SecretsBootError> {
    let Some(secret_name) = cfg.database_password_secret.as_deref() else {
        return Ok(cfg.database_url.clone());
    };

    let dsn = cfg
        .database_url
        .as_deref()
        .ok_or(SecretsBootError::NoDatabaseUrl)?;

    let password = store
        .get(secret_name)
        .map_err(|source| SecretsBootError::StoreRead {
            name: secret_name.to_owned(),
            source,
        })?
        .ok_or_else(|| SecretsBootError::SecretMissing {
            name: secret_name.to_owned(),
        })?;

    let spliced = splice_password(dsn, password.expose())?;
    Ok(Some(spliced))
}

/// Parse `dsn`, set its password to `password`, and re-serialize.
fn splice_password(dsn: &str, password: &str) -> Result<String, SecretsBootError> {
    let mut url = url::Url::parse(dsn).map_err(|source| SecretsBootError::BadDsn {
        dsn: dsn.to_owned(),
        source,
    })?;
    url.set_password(Some(password))
        .map_err(|()| SecretsBootError::SetPassword {
            dsn: dsn.to_owned(),
        })?;
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::secrets::{Secret, SecretError};

    /// Minimal in-memory store returning a fixed password for one key.
    struct MockStore {
        key: String,
        value: Option<String>,
    }

    impl SecretStore for MockStore {
        fn ready(&self) -> bool {
            true
        }

        fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
            if name == self.key {
                Ok(self.value.clone().map(Secret::new))
            } else {
                Ok(None)
            }
        }

        fn put(&self, _name: &str, _value: Secret) -> Result<(), SecretError> {
            Ok(())
        }

        fn delete(&self, _name: &str) -> Result<(), SecretError> {
            Ok(())
        }
    }

    fn cfg_with(database_url: Option<&str>, secret: Option<&str>) -> AgentConfig {
        AgentConfig {
            database_url: database_url.map(str::to_owned),
            database_password_secret: secret.map(str::to_owned),
            ..AgentConfig::default()
        }
    }

    #[test]
    fn splices_password_from_store() {
        let store = MockStore {
            key: "db:password".to_owned(),
            value: Some("hunter2".to_owned()),
        };
        let cfg = cfg_with(
            Some("postgres://rubix@127.0.0.1:5433/rubix"),
            Some("db:password"),
        );
        let resolved = resolve_database_url(&cfg, &store).unwrap().unwrap();
        assert_eq!(resolved, "postgres://rubix:hunter2@127.0.0.1:5433/rubix");
    }

    #[test]
    fn replaces_existing_password() {
        let store = MockStore {
            key: "db:password".to_owned(),
            value: Some("real-secret".to_owned()),
        };
        let cfg = cfg_with(
            Some("postgres://rubix:placeholder@host:5432/rubix"),
            Some("db:password"),
        );
        let resolved = resolve_database_url(&cfg, &store).unwrap().unwrap();
        assert_eq!(resolved, "postgres://rubix:real-secret@host:5432/rubix");
    }

    #[test]
    fn passes_dsn_through_when_no_secret_configured() {
        let store = MockStore {
            key: "db:password".to_owned(),
            value: Some("hunter2".to_owned()),
        };
        let cfg = cfg_with(Some("postgres://rubix:plain@host:5432/rubix"), None);
        let resolved = resolve_database_url(&cfg, &store).unwrap().unwrap();
        assert_eq!(resolved, "postgres://rubix:plain@host:5432/rubix");
    }

    #[test]
    fn missing_secret_is_a_hard_error() {
        let store = MockStore {
            key: "db:password".to_owned(),
            value: None,
        };
        let cfg = cfg_with(Some("postgres://rubix@host:5432/rubix"), Some("db:password"));
        let err = resolve_database_url(&cfg, &store).unwrap_err();
        assert!(matches!(err, SecretsBootError::SecretMissing { .. }));
    }

    #[test]
    fn secret_without_dsn_is_an_error() {
        let store = MockStore {
            key: "db:password".to_owned(),
            value: Some("hunter2".to_owned()),
        };
        let cfg = cfg_with(None, Some("db:password"));
        let err = resolve_database_url(&cfg, &store).unwrap_err();
        assert!(matches!(err, SecretsBootError::NoDatabaseUrl));
    }
}
