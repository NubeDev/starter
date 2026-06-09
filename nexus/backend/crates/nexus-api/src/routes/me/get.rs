//! `GET /api/v1/me` — the frontend's identity-context endpoint.
//!
//! Fills the gap left by `starter-auth-users`' `/auth/me`, which returns only
//! `{subject, email, role}`. This returns the tenant binding, team memberships,
//! and scopes the SPA needs for `usePrincipal()` / `useCan()` without a
//! round-trip per resource. Reads the `Principal` the auth middleware injected.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use nexus_spi::dto::me::MeResponse;
use starter_spi::auth::Principal;

/// Map the authenticated `Principal` to the richer context DTO. 401 when no
/// principal is present (the request was unauthenticated).
#[utoipa::path(
    get,
    path = "/api/v1/me",
    tag = "me",
    operation_id = "get_me",
    responses(
        (status = 200, description = "Caller context", body = MeResponse),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn get_me(principal: Option<Extension<Principal>>) -> axum::response::Response {
    let Some(Extension(p)) = principal else {
        return (StatusCode::UNAUTHORIZED, "unauthenticated").into_response();
    };
    Json(MeResponse {
        subject: p.subject,
        role: format!("{:?}", p.role).to_lowercase(),
        tenant_id: p.tenant_id,
        teams: p.teams,
        scopes: p.scopes.into_iter().map(|s| s.0).collect(),
    })
    .into_response()
}
