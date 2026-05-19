//! `SecretStore` — the seam over OS keyring, age-encrypted file,
//! or any future backend.
//!
//! # Sync vs async (SCOPE open question 5)
//!
//! Trait is **sync**. The two shipped backends
//! ([`starter-secrets-keyring`](https://docs.rs/starter-secrets-keyring)
//! and [`starter-secrets-file`](https://docs.rs/starter-secrets-file))
//! are both sync at the bottom — the OS keyring API is sync, the file
//! backend reads a few KB through `std::fs`. An async trait would
//! force every consumer of `SecretStore` into the tokio runtime even
//! when neither side benefits.
//!
//! Secrets are read at startup and on rare config changes, not on the
//! hot path of every request — the cost of a blocking call is
//! negligible. If a future network-backed impl ships (HashiCorp Vault,
//! AWS Secrets Manager), it should:
//!
//! 1. Cache aggressively at construction time so steady-state calls
//!    are local.
//! 2. Use `tokio::task::block_in_place` on the rare cold path, OR
//!    ship an `AsyncSecretStore` parallel trait — the spi-internal
//!    decision happens then, not now.
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
#[non_exhaustive]
pub enum SecretError {
    /// Backend is present but rejected the operation (permissions,
    /// locked keyring, decryption failure).
    #[error("secret store backend error: {0}")]
    Backend(String),

    /// I/O failure reaching the backend.
    #[error("secret store I/O error: {0}")]
    Io(#[from] std::io::Error),
}
