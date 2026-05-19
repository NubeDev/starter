//! Authenticated identity. Carries enough context for downstream
//! authorization checks; consumers extend via the `extra` bag for
//! their own claims.

use serde::{Deserialize, Serialize};

use super::{Role, Scope};

/// Who the request is being made by, after the `Authenticator`
/// verifies credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principal {
    /// Stable subject identifier (typically the user id or, for
    /// `starter-auth-token`, the claim id).
    pub subject: String,

    /// Coarse permission level. Drives `require_role` middleware.
    pub role: Role,

    /// Fine-grained scopes attached to the credential. Drives
    /// `require_scope` middleware.
    pub scopes: Vec<Scope>,

    /// Consumer-defined extra claims. Keep this small — heavy
    /// per-request lookups belong elsewhere.
    #[serde(default)]
    pub extra: serde_json::Value,
}
