//! Detection CRUD handlers.
//!
//! LAYER: transport (REST). Extract → authorize → call store → shape DTO. The
//! authorization mirrors the alert-rule and insight handlers: list is unfiltered
//! within the tenant (RLS already isolates it), and per-id reads/writes check
//! the standard view/edit/delete grant on `KIND_DETECTION`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::detection::{
    CreateDetectionRequest, DetectionDetail, DetectionStats, UpdateDetectionRequest,
};
use nexus_store::detection::{self, DetectionPatch, NewDetection};
use serde_json::{json, Value};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::{detection_to_detail, stats_to_dto};
use crate::authz::{self, ACTION_DELETE, ACTION_EDIT, ACTION_VIEW, KIND_DETECTION};
use crate::middleware::tenant::{caller, tenant_of};
use crate::state::AppState;

#[utoipa::path(get, path = "/api/v1/detections", tag = "detections", operation_id = "list_detections",
    responses((status = 200, description = "Detections", body = [DetectionDetail])))]
pub async fn list_detections(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match detection::list(&state.metadata, &tenant).await {
        Ok(ds) => Json(ds.iter().map(detection_to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(post, path = "/api/v1/detections", tag = "detections", operation_id = "create_detection",
    request_body = CreateDetectionRequest,
    responses((status = 200, description = "Created", body = DetectionDetail),
        (status = 400, description = "References a missing insight")))]
pub async fn create_detection(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateDetectionRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewDetection {
        name: req.name,
        insight_id: req.insight_id,
        datasource_id: req.datasource_id,
        sql: req.sql,
        params: req.params.unwrap_or_else(|| Value::Object(Default::default())),
        sources: req.sources.unwrap_or_else(|| Value::Array(Vec::new())),
        flag_column: req.flag_column,
        target_columns: req.target_columns,
        value_column: req.value_column,
        for_secs: req.for_secs.unwrap_or(0),
        interval_secs: req.interval_secs.unwrap_or(300),
        enabled: req.enabled.unwrap_or(true),
    };
    match detection::insert(&state.metadata, &tenant, &new).await {
        Ok(d) => Json(detection_to_detail(&d)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/detections/{id}", tag = "detections", operation_id = "get_detection",
    params(("id" = Uuid, Path, description = "Detection id")),
    responses((status = 200, description = "Detection", body = DetectionDetail), (status = 404, description = "Not found")))]
pub async fn get_detection(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let d = match detection::get(&state.metadata, &tenant, id).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_VIEW, KIND_DETECTION, &id.to_string(), &tenant).await {
        return resp;
    }
    Json(detection_to_detail(&d)).into_response()
}

#[utoipa::path(put, path = "/api/v1/detections/{id}", tag = "detections", operation_id = "update_detection",
    params(("id" = Uuid, Path, description = "Detection id")), request_body = UpdateDetectionRequest,
    responses((status = 204, description = "Updated"), (status = 404, description = "Not found")))]
pub async fn update_detection(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDetectionRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_EDIT, KIND_DETECTION, &id.to_string(), &tenant).await {
        return resp;
    }
    let patch = DetectionPatch {
        name: req.name,
        insight_id: req.insight_id,
        // clear wins over set: clear ⇒ Some(None) (dev pool); a uuid ⇒
        // Some(Some(id)); neither ⇒ None (unchanged). Mirrors the panel
        // insight_id/clear_insight translation.
        datasource_id: if req.clear_datasource {
            Some(None)
        } else {
            req.datasource_id.map(Some)
        },
        sql: req.sql,
        params: req.params,
        sources: req.sources,
        flag_column: req.flag_column,
        target_columns: req.target_columns,
        value_column: req.value_column,
        for_secs: req.for_secs,
        interval_secs: req.interval_secs,
        enabled: req.enabled,
    };
    match detection::update(&state.metadata, &tenant, id, &patch).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(delete, path = "/api/v1/detections/{id}", tag = "detections", operation_id = "delete_detection",
    params(("id" = Uuid, Path, description = "Detection id")),
    responses((status = 204, description = "Deleted (findings cascade)"), (status = 404, description = "Not found")))]
pub async fn delete_detection(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_DELETE, KIND_DETECTION, &id.to_string(), &tenant).await {
        return resp;
    }
    match detection::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Manually run a detection now, outside its schedule — the deterministic seam
/// the WS acceptance leans on ("runs on its interval … verified the same way the
/// alert scheduler was"). Returns the upsert/resolve counts. Requires `edit`.
#[utoipa::path(post, path = "/api/v1/detections/{id}/run", tag = "detections", operation_id = "run_detection_now",
    params(("id" = Uuid, Path, description = "Detection id")),
    responses((status = 200, description = "Ran", body = Value), (status = 404, description = "Not found")))]
pub async fn run_now(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_EDIT, KIND_DETECTION, &id.to_string(), &tenant).await {
        return resp;
    }
    if detection::get(&state.metadata, &tenant, id).await.ok().flatten().is_none() {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    let ctx = crate::detecting::run::RunContext {
        state: &state,
        metadata: &state.metadata,
        envelope: &state.envelope,
        pools: &state.datasource_pools,
        dev_pool: &state.datasource,
        guards: state.guards,
    };
    crate::detecting::run::run_detection(&ctx, &tenant, id).await;
    Json(json!({ "ran": true })).into_response()
}

/// Run stats for a detection: next run time + its findings by status. A
/// glanceable summary for the list and editor. Requires `view`.
#[utoipa::path(get, path = "/api/v1/detections/{id}/stats", tag = "detections", operation_id = "detection_stats",
    params(("id" = Uuid, Path, description = "Detection id")),
    responses((status = 200, description = "Stats", body = DetectionStats), (status = 404, description = "Not found")))]
pub async fn get_stats(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(state.engine.as_ref(), caller, ACTION_VIEW, KIND_DETECTION, &id.to_string(), &tenant).await {
        return resp;
    }
    match detection::stats(&state.metadata, &tenant, id).await {
        Ok(Some(s)) => Json(stats_to_dto(&s)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
