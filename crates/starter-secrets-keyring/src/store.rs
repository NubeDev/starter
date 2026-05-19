//! `KeyringSecretStore` — `SecretStore` impl over the OS keyring.

use keyring::{Entry, Error as KeyringError};
use starter_spi::secrets::{Secret, SecretError, SecretStore};

/// `SecretStore` over the OS keyring. The service name is the
/// consumer's binary crate name; per-key user strings are namespaced
/// `<binary>:<name>` so two starter-based apps do not collide.
#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    binary: String,
}

impl KeyringSecretStore {
    /// Build a store for `binary` (e.g. the crate name of the
    /// consumer's binary).
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    fn entry(&self, name: &str) -> Result<Entry, SecretError> {
        let user = format!("{}:{name}", self.binary);
        Entry::new(&self.binary, &user).map_err(map_err)
    }
}

impl SecretStore for KeyringSecretStore {
    fn ready(&self) -> bool {
        // A successful `Entry::new` plus a benign `get_password` lookup
        // exercises the platform service end to end. `NoEntry` is the
        // only "service is up, key just doesn't exist" outcome — treat
        // it as ready. Everything else (NoStorageAccess, PlatformFailure
        // on Linux when DBus isn't running) means the backend can't
        // serve.
        let Ok(entry) = self.entry("__starter_probe__") else {
            return false;
        };
        matches!(entry.get_password(), Ok(_) | Err(KeyringError::NoEntry))
    }

    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError> {
        let entry = self.entry(name)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(Secret::new(value))),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(map_err(e)),
        }
    }

    fn put(&self, name: &str, value: Secret) -> Result<(), SecretError> {
        let entry = self.entry(name)?;
        entry.set_password(value.expose()).map_err(map_err)
    }

    fn delete(&self, name: &str) -> Result<(), SecretError> {
        let entry = self.entry(name)?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(map_err(e)),
        }
    }
}

fn map_err(e: KeyringError) -> SecretError {
    SecretError::Backend(e.to_string())
}
