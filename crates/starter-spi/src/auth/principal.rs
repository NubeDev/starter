//! Authenticated identity. Carries enough context for downstream
//! authorization checks; consumers extend via the `extra` bag for
//! their own claims.

use serde::{Deserialize, Serialize};

/// Who the request is being made by, after the `Authenticator`
/// verifies credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    /// Stable subject identifier (typically the `sub` JWT claim).
    pub subject: String,

    /// Scopes / permissions granted to this principal.
    pub scopes: Vec<String>,

    /// Consumer-defined extra claims. Keep this small — heavy
    /// per-request lookups belong elsewhere.
    pub extra: serde_json::Value,
}
