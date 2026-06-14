//! Resolve the tenant a request acts within, from its authenticated principal.
//!
//! Every tenant-scoped handler binds its store calls to this tenant, which the
//! RLS layer then enforces. A principal with no tenant binding cannot act on
//! tenant-scoped resources — it is rejected rather than defaulted, so a missing
//! binding fails closed.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use starter_spi::auth::Principal;

/// The tenant a request acts within. Rejects an unauthenticated caller (401) and
/// a caller with no tenant binding (403) — the latter is authenticated but not
/// scoped to any tenant, so it has nothing to act on here.
// The error is an axum `Response` (the early-return pattern), intentionally
// larger than the `String` success value — not a case for boxing.
#[allow(clippy::result_large_err)]
pub fn tenant_of(principal: &Option<axum::Extension<Principal>>) -> Result<String, Response> {
    Ok(caller(principal)?.1)
}

/// Both halves a tenant-scoped, grant-checked handler needs: the authenticated
/// `Principal` (for the grant check) and its tenant string (for store scoping).
/// Rejects an unauthenticated caller (401) or one with no tenant binding (403)
/// with the same fail-closed semantics as [`tenant_of`].
#[allow(clippy::result_large_err)]
pub fn caller(
    principal: &Option<axum::Extension<Principal>>,
) -> Result<(&Principal, String), Response> {
    let Some(axum::Extension(p)) = principal else {
        return Err((StatusCode::UNAUTHORIZED, "unauthenticated").into_response());
    };
    match p.tenant_id.as_deref() {
        Some(t) if !t.is_empty() => Ok((p, t.to_string())),
        _ => Err((StatusCode::FORBIDDEN, "no tenant binding").into_response()),
    }
}
