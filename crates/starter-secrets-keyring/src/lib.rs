//! # starter-secrets-keyring
//!
//! `SecretStore` impl backed by the OS keyring (macOS Keychain,
//! Windows Credential Manager, Linux Secret Service via DBus).
//! Intended for desktop / developer workstations.
//!
//! ## Namespacing (SCOPE 556–563)
//!
//! Service name = the consumer binary's crate name (passed to
//! [`KeyringSecretStore::new`]). Each stored entry's "user" field is
//! formatted as `<binary>:<starter-component>:<key>` so two starter-
//! based apps on the same machine do not collide. Names that callers
//! pass in (e.g. `"auth-token:pending"`, `"ai:anthropic:api_key"`)
//! are taken as the `<starter-component>:<key>` portion and prefixed
//! with the binary name at write time.
//!
//! ## CI / headless caveat
//!
//! Server VMs and CI runners have no keyring daemon. [`ready`] probes
//! the backend with a throw-away `Entry::get_password` call: if the
//! platform service is missing (no DBus on Linux, etc.) it returns
//! `false`. Consumers detect this and feature-swap to
//! `starter-secrets-file`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod store;

pub use store::KeyringSecretStore;
