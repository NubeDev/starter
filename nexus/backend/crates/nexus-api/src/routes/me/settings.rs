//! `GET`/`PUT /api/v1/me/settings` — the caller's freeform settings bag.
//!
//! LAYER: transport. Reads the `Principal` the auth layer injected and pins the
//! row to `(user_id = principal.subject, tenant_id = principal.tenant_id)`, so a
//! caller can only ever read or write its own settings within its own tenant.
//!
//! The bag is opaque nexus-side UI state the frontend owns (starred dashboards,
//! collapsed groups, …). `PUT` is a full replace, mirroring the tag editor: the
//! client reads, modifies, and writes the whole bag, so the store never merges.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::response::Response;
use axum::{Extension, Json};
use nexus_spi::dto::me::UserSettings;
use nexus_store::user_settings;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use crate::state::AppState;

/// `GET /api/v1/me/settings` — the caller's settings bag, `{}` when never saved.
#[utoipa::path(
    get,
    path = "/api/v1/me/settings",
    tag = "me",
    operation_id = "get_me_settings",
    responses(
        (status = 200, description = "The caller's settings", body = UserSettings),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn get_me_settings(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> Response {
    let Some((user_id, tenant)) = keys(principal) else {
        return unauthorized();
    };
    match user_settings::get(&state.metadata, &tenant, &user_id).await {
        Ok(settings) => Json(UserSettings { settings }).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// `PUT /api/v1/me/settings` — replace the caller's settings bag (full replace).
#[utoipa::path(
    put,
    path = "/api/v1/me/settings",
    tag = "me",
    operation_id = "set_me_settings",
    request_body = UserSettings,
    responses(
        (status = 200, description = "The caller's settings after the write", body = UserSettings),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn set_me_settings(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(body): Json<UserSettings>,
) -> Response {
    let Some((user_id, tenant)) = keys(principal) else {
        return unauthorized();
    };
    match user_settings::set(&state.metadata, &tenant, &user_id, &body.settings).await {
        Ok(()) => Json(body).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Pull `(user_id, tenant_id)` from the principal, requiring a tenant binding.
/// `user_id` is the subject; the tenant scopes the store and RLS. Same pinning
/// as the preferences handlers, so settings ride the caller's own rows.
fn keys(principal: Option<Extension<Principal>>) -> Option<(String, String)> {
    let Extension(p) = principal?;
    let tenant = p.tenant_id.as_deref().filter(|t| !t.is_empty())?;
    Some((p.subject, tenant.to_string()))
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "unauthenticated").into_response()
}
