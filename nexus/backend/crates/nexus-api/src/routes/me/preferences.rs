//! `GET`/`PATCH /api/v1/me/preferences` — the caller's resolved preferences.
//!
//! LAYER: transport. Reads the `Principal` the auth layer injected and pins the
//! preference workspace to its tenant (`workspace_id = principal.tenant_id`), so
//! a caller can only ever read or write its own tenant's rows. This is the
//! isolation contract for the reused `starter-prefs` store, which runs outside
//! `tenant_tx` and so carries no `app.tenant_id` RLS GUC (see `1501_prefs.sql`).
//!
//! The three-layer merge (user → org → default) and the storage round-trip are
//! `starter-prefs`' job; this module only binds them to nexus tenancy and shapes
//! the wire surface. PATCH bodies are parsed as a raw JSON object so a missing
//! key ("leave alone") stays distinct from an explicit `null` ("revert to
//! inherit") — the same distinction `starter-prefs`' own routes draw.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde_json::{Map, Value as JsonValue};
use starter_prefs::resolver::resolve;
use starter_prefs::store::PrefsStore as _;
use starter_spi::auth::Principal;
use starter_spi::preferences::{PreferencesPatch, ResolvedPreferences};

use crate::state::AppState;

use super::prefs_apply::apply_user_patch;

/// `GET /api/v1/me/preferences` — resolve the caller's user → org → default
/// preferences for its own tenant. 401 without a tenant-bound principal.
#[utoipa::path(
    get,
    path = "/api/v1/me/preferences",
    tag = "me",
    operation_id = "get_me_preferences",
    responses(
        (status = 200, description = "Resolved preferences", body = ResolvedPreferences),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn get_me_preferences(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> Response {
    let Some((user_id, workspace_id)) = keys(principal) else {
        return unauthorized();
    };
    let user = match state
        .prefs
        .store
        .get_user_prefs(&user_id, &workspace_id)
        .await
    {
        Ok(u) => u,
        Err(e) => return internal(e),
    };
    let org = match state.prefs.store.get_org_prefs(&workspace_id).await {
        Ok(o) => o,
        Err(e) => return internal(e),
    };
    Json(resolve(user, org, &state.prefs.defaults)).into_response()
}

/// `PATCH /api/v1/me/preferences` — merge a partial update into the caller's
/// user-layer row for its own tenant. Missing key leaves a field unchanged;
/// explicit `null` reverts it to inherit. 401 without a tenant-bound principal.
#[utoipa::path(
    patch,
    path = "/api/v1/me/preferences",
    tag = "me",
    operation_id = "patch_me_preferences",
    request_body = PreferencesPatch,
    responses(
        (status = 200, description = "Resolved preferences after patch", body = ResolvedPreferences),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn patch_me_preferences(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(patch): Json<Map<String, JsonValue>>,
) -> Response {
    let Some((user_id, workspace_id)) = keys(principal) else {
        return unauthorized();
    };
    let mut row = match state
        .prefs
        .store
        .get_user_prefs(&user_id, &workspace_id)
        .await
    {
        Ok(r) => r.unwrap_or_default(),
        Err(e) => return internal(e),
    };
    if let Err(detail) = apply_user_patch(&mut row, &patch) {
        return (StatusCode::BAD_REQUEST, detail).into_response();
    }
    if let Err(e) = state
        .prefs
        .store
        .upsert_user_prefs(&user_id, &workspace_id, row.clone())
        .await
    {
        return internal(e);
    }
    let org = match state.prefs.store.get_org_prefs(&workspace_id).await {
        Ok(o) => o,
        Err(e) => return internal(e),
    };
    Json(resolve(Some(row), org, &state.prefs.defaults)).into_response()
}

/// Pull `(user_id, workspace_id)` from the principal, requiring a tenant
/// binding. `user_id` is the subject; `workspace_id` is the tenant — the same
/// pinning the units middleware uses, so resolved units match the rows the
/// caller can read and write.
fn keys(principal: Option<Extension<Principal>>) -> Option<(String, String)> {
    let Extension(p) = principal?;
    let tenant = p.tenant_id.as_deref().filter(|t| !t.is_empty())?;
    Some((p.subject, tenant.to_string()))
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "unauthenticated").into_response()
}

fn internal<E: std::fmt::Display>(err: E) -> Response {
    tracing::warn!(error = %err, "prefs store error");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
}
