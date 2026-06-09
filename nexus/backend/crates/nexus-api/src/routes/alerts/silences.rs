//! Silence (maintenance-window) handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::alert::{CreateSilenceRequest, SilenceDetail};
use nexus_store::alert::{silence, NewSilence};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::silence_to_detail;
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(get, path = "/api/v1/alerts/silences", tag = "alerts", operation_id = "list_silences",
    responses((status = 200, description = "Silences", body = [SilenceDetail])))]
pub async fn list_silences(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (_caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match silence::list(&state.metadata, &tenant).await {
        Ok(ss) => Json(ss.iter().map(silence_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/alerts/silences", tag = "alerts", operation_id = "create_silence",
    request_body = CreateSilenceRequest,
    responses((status = 200, description = "Created", body = SilenceDetail)))]
pub async fn create_silence(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateSilenceRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let new = NewSilence {
        rule_id: req.rule_id,
        starts_at: req.starts_at,
        ends_at: req.ends_at,
        reason: req.reason,
        created_by: caller.subject.clone(),
    };
    match silence::insert(&state.metadata, &tenant, &new).await {
        Ok(s) => Json(silence_to_detail(&s)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/alerts/silences/{id}", tag = "alerts", operation_id = "delete_silence",
    params(("id" = Uuid, Path, description = "Silence id")),
    responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_silence(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (_caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match silence::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
