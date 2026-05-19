//! `SecretStore` — the seam over OS keyring, age-encrypted file,
//! or any future backend. Sync on purpose; the keyring backend is
//! sync and async-wrapping it would force every consumer into the
//! tokio runtime.
//!
//! See SCOPE.md 342–346, 562–563.

use super::secret::Secret;

/// Read / write / delete named secrets.
///
/// Names are convention-based, dotted by component: e.g.
/// `auth-token:pending`, `ai:anthropic:api_key`. Each impl decides
/// its own namespacing on disk.
pub trait SecretStore: Send + Sync + 'static {
    /// `true` when the backend is wired and a `get`/`put` is
    /// expected to succeed. The keyring backend returns `false` in
    /// headless / CI environments; consumers should fall through to
    /// env-var or file fallbacks.
    fn ready(&self) -> bool;

    /// Fetch the secret stored under `name`. `Ok(None)` means "no
    /// such key"; `Err` means the backend failed.
    fn get(&self, name: &str) -> Result<Option<Secret>, SecretError>;

    /// Store `value` at `name`, overwriting any prior value.
    fn put(&self, name: &str, value: Secret) -> Result<(), SecretError>;

    /// Remove the value at `name`. Removing a missing key is not an
    /// error.
    fn delete(&self, name: &str) -> Result<(), SecretError>;
}

/// Failure modes any `SecretStore` impl can surface.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    /// Backend is present but rejected the operation (permissions,
    /// locked keyring, decryption failure).
    #[error("secret store backend error: {0}")]
    Backend(String),

    /// I/O failure reaching the backend.
    #[error("secret store I/O error: {0}")]
    Io(#[from] std::io::Error),
}
