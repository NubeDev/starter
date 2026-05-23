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

    /// Tenant the session is bound to (Phase 7a — R11). `None`
    /// means the authenticator did not bind the principal to any
    /// tenant; rules and resources that opt into tenancy
    /// (`ResourceSpec.tenant_scoped = true`) deny such a principal
    /// with `reason = "no_tenant_binding"`. Consumers running
    /// pre-Phase-7 wiring leave this at `None` and never set
    /// `tenant_scoped`, so the predicate never fires.
    ///
    /// The super-admin sentinel value `"*"` is reserved: it
    /// bypasses the cross-tenant predicate (used by API tokens
    /// minted for cross-tenant administration). The reserved
    /// sentinel is only allowed for principals whose role is
    /// `Admin`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Consumer-defined extra claims. Keep this small — heavy
    /// per-request lookups belong elsewhere.
    #[serde(default)]
    pub extra: serde_json::Value,
}

impl Principal {
    /// Convenience: is this principal a super-admin (tenant_id ==
    /// `"*"`)? Engines use this to short-circuit the cross-tenant
    /// predicate.
    pub fn is_super_admin(&self) -> bool {
        matches!(self.tenant_id.as_deref(), Some("*"))
    }
}
