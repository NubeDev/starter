//! `Secret` — a small newtype around a string that the secret-store
//! impls return. Display/Debug deliberately redact the value so a
//! stray `tracing::info!("{:?}", secret)` cannot leak it.

use serde::{Deserialize, Serialize};

/// An opaque secret value. Treat as write-once / pass-through; do not
/// log, do not derive `Eq` for comparison in error messages.
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    /// Wrap a string as a secret. Caller is responsible for ensuring
    /// the source string is not retained or logged.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the underlying value. Use sparingly — every call site
    /// is a place the secret can leak.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Consume the secret and return the inner string. Same caveat
    /// as `expose`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}
