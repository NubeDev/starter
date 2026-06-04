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

    /// Teams the principal belongs to in the current tenant
    /// (Phase 7b — R13). Slug list, not ids: rules reference
    /// teams by stable slug (`principal.teams contains
    /// "hvac-ops"`); UUIDs would force rule edits whenever a team
    /// is recreated. Populated by the authenticator at session-
    /// mint / token-verify time from
    /// `starter_auth_users_team_members` joined to
    /// `starter_auth_users_teams.slug`. Empty (`[]`) for any
    /// principal that pre-dates Phase 7b or that the
    /// authenticator did not look up team memberships for —
    /// conditions referencing `principal.teams` simply do not
    /// match, which keeps Phase 1–6 wiring unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teams: Vec<String>,

    /// Tenant ids this principal administers — the subtree rooted at
    /// `tenant_id`, inclusive (ADR-tenant-hierarchy). Resolved at
    /// session-mint / token-verify time from the tenant closure
    /// table. For a leaf tenant this is just `[tenant_id]`; for a
    /// parent (e.g. a reseller) it also contains every descendant
    /// tenant id so the engine's cross-tenant predicate admits the
    /// whole subtree.
    ///
    /// Empty (`[]`) for any principal that pre-dates the hierarchy
    /// work or whose authenticator did not look the subtree up — the
    /// engine then falls back to strict `tenant_id == object.tenant`
    /// equality, preserving the flat-tenant behaviour (SCOPE-EXT.md
    /// R11) byte-for-byte. The `"*"` super-admin sentinel leaves this
    /// empty and short-circuits via [`Principal::is_super_admin`]
    /// instead (the whole-forest case).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tenant_scope: Vec<String>,

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

    /// Does this principal administer `tenant`? True when `tenant`
    /// is the principal's own tenant (depth-0) or any tenant in its
    /// resolved subtree (`tenant_scope`). The cross-tenant predicate
    /// uses this to admit a parent acting on a descendant's resource
    /// (ADR-tenant-hierarchy). Does **not** consider the `"*"`
    /// sentinel — callers check [`Principal::is_super_admin`] first.
    pub fn administers_tenant(&self, tenant: &str) -> bool {
        self.tenant_id.as_deref() == Some(tenant)
            || self.tenant_scope.iter().any(|t| t == tenant)
    }
}
