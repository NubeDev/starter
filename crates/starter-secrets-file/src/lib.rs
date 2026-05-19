//! # starter-secrets-file
//!
//! `SecretStore` impl backed by a single
//! [age](https://age-encryption.org)-encrypted file at
//! `$XDG_DATA_HOME/<binary>/secrets.age` (SCOPE 565–572).
//!
//! The on-disk format is ASCII-armored age over a JSON object
//! (`{ name -> value }`). One file, one identity, one read/write lock.
//! Intended for headless servers and containers where the OS keyring
//! is not reachable.
//!
//! ## Identity resolution
//!
//! On first use, the store resolves an X25519 identity in this order:
//!
//! 1. `STARTER_SECRETS_KEY` env var — the value is the AGE-SECRET-KEY
//!    string directly.
//! 2. The consumer's config path, passed in via
//!    [`FileSecretStoreBuilder::identity_path`]. The file contents
//!    are parsed as an age identity.
//! 3. **First-run generation** — a fresh identity is created, written
//!    next to `secrets.age` as `identity.age-key`, and a one-time
//!    `tracing::warn!` prints both the identity public string and the
//!    file path so the operator can back it up. If they lose this
//!    key, the secrets file is unrecoverable.
//!
//! ## Not in scope
//!
//! No HSM, no cloud KMS, no key rotation. A consumer needing those
//! writes their own `SecretStore` impl behind the trait (SCOPE
//! 775–778).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod store;

pub use store::{FileSecretStore, FileSecretStoreBuilder, FileSecretsError};
