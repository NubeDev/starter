//! The audit-read privilege gate.
//!
//! Audit is privileged (WS-12 §3.4): only an admin may read the log, and only
//! within a tenant they administer. The tenant pin then flows into
//! [`nexus_store::changelog::NexusChangeLog`], so RLS is the second layer — a
//! non-admin is refused here before any row is read, and even an admin only ever
//! sees their own tenant's rows because the log binds `app.tenant_id`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use starter_spi::auth::{Principal, Role};

/// Resolve the tenant an admin caller may read audit for. Rejects an
/// unauthenticated caller (401), a non-admin (403 `audit_read_forbidden`), and an
/// admin with no tenant binding (403 `no_tenant_binding`). Returns the tenant the
/// log is pinned to — the caller's own tenant. A super-admin (`tenant_id == "*"`)
/// has no single tenant to pin, so cross-tenant audit is out of scope for this
/// route and such a caller is asked to read a concrete tenant's API instead.
// The early-return error is an axum `Response`, intentionally larger than the
// `String` success value — the established pattern in this crate, not a boxing case.
#[allow(clippy::result_large_err)]
pub fn require_audit_read(
    principal: &Option<Extension<Principal>>,
) -> Result<String, Response> {
    let Some(Extension(p)) = principal else {
        return Err((StatusCode::UNAUTHORIZED, "unauthenticated").into_response());
    };
    if p.role != Role::Admin {
        return Err((StatusCode::FORBIDDEN, "audit_read_forbidden").into_response());
    }
    match p.tenant_id.as_deref() {
        Some("*") => Err((
            StatusCode::FORBIDDEN,
            "audit read requires a concrete tenant binding",
        )
            .into_response()),
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err((StatusCode::FORBIDDEN, "no_tenant_binding").into_response()),
    }
}
