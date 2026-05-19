//! Secret-store seam. Implementations live in
//! `starter-secrets-keyring` and `starter-secrets-file`; this module
//! defines the trait everything programs against.

mod secret;
mod store;

pub use secret::Secret;
pub use store::{SecretError, SecretStore};
