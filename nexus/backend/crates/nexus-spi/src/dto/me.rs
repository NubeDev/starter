//! `GET /api/v1/me` — the frontend's identity-context endpoint.
//!
//! Fills the gap left by `starter-auth-users`' `/auth/me`, which returns only
//! `{subject, email, role}`. The SPA's `usePrincipal()` / `useCan()` need the
//! tenant binding, team memberships, and the coarse permission set up front so
//! it can gate navigation without a round-trip per resource.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The authenticated caller's full context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MeResponse {
    /// Stable subject id (the user id).
    pub subject: String,
    /// Coarse role: `reader` | `writer` | `admin`.
    pub role: String,
    /// Tenant the session is bound to. `None` for an unbound principal; `"*"`
    /// marks a super-admin that bypasses the cross-tenant predicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Team slugs the caller belongs to in the current tenant. Grants reference
    /// teams by slug, so the SPA matches against these.
    pub teams: Vec<String>,
    /// Fine-grained scope strings attached to the credential.
    pub scopes: Vec<String>,
}

/// The caller's freeform settings bag — `GET`/`PUT /api/v1/me/settings`.
///
/// An opaque envelope around the per-user `jsonb` the frontend owns (starred
/// dashboards, collapsed sidebar groups, …). The server neither defines nor
/// validates the keys; `PUT` is a full replace, so the client reads, modifies,
/// and writes the whole bag. `settings` is always an object (defaults to `{}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UserSettings {
    pub settings: Value,
}
