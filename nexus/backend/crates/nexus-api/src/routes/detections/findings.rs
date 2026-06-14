//! Findings browse + lifecycle handlers.
//!
//! A finding inherits its detection's grant: `view` to read, `edit` to
//! acknowledge or resolve. The list is filtered to detections the caller may
//! view (RLS isolates the tenant; the per-detection grant is checked when a
//! `detection_id` filter is given, matching the alert-events browse posture).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use nexus_spi::dto::detection::{Finding, FindingActionRequest};
use nexus_store::finding::{self, FindingFilter};
use serde::Deserialize;
use serde_json::Value;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::finding_to_dto;
use crate::authz::{self, ACTION_EDIT, ACTION_VIEW, KIND_DETECTION};
use crate::middleware::tenant::caller;
use crate::state::AppState;

/// Server-side cap on a findings page, matching the alert-events browse cap.
const FINDING_LIMIT: i64 = 200;

/// Query filters for the findings feed. `target` is a JSON object string the
/// `target` column must contain (e.g. `{"site":"s1"}`), enabling site/meter
/// filtering without a fixed target schema.
#[derive(Debug, Default, Deserialize)]
pub struct FindingQuery {
    pub detection_id: Option<Uuid>,
    pub status: Option<String>,
    pub target: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/findings", tag = "findings", operation_id = "list_findings",
    responses((status = 200, description = "Findings", body = [Finding])))]
pub async fn list_findings(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Query(q): Query<FindingQuery>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // When scoped to one detection, require view on it; otherwise the tenant-wide
    // feed is RLS-isolated already.
    if let Some(d) = q.detection_id {
        if let Err(resp) = authz::require(
            state.engine.as_ref(),
            caller,
            ACTION_VIEW,
            KIND_DETECTION,
            &d.to_string(),
            &tenant,
        )
        .await
        {
            return resp;
        }
    }
    let target_contains = match q.target.as_deref().map(serde_json::from_str::<Value>) {
        Some(Ok(v)) => Some(v),
        Some(Err(_)) => {
            return (StatusCode::BAD_REQUEST, "target must be a JSON object").into_response()
        }
        None => None,
    };
    let filter = FindingFilter {
        detection_id: q.detection_id,
        status: q.status,
        target_contains,
        since: q.since,
        until: q.until,
        limit: q.limit.unwrap_or(FINDING_LIMIT).clamp(1, FINDING_LIMIT),
    };
    match finding::list(&state.metadata, &tenant, &filter).await {
        Ok(fs) => Json(fs.iter().map(finding_to_dto).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(get, path = "/api/v1/findings/{id}", tag = "findings", operation_id = "get_finding",
    params(("id" = Uuid, Path, description = "Finding id")),
    responses((status = 200, description = "Finding", body = Finding), (status = 404, description = "Not found")))]
pub async fn get_finding(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let f = match finding::get(&state.metadata, &tenant, id).await {
        Ok(Some(f)) => f,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_DETECTION,
        &f.detection_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(finding_to_dto(&f)).into_response()
}

#[utoipa::path(post, path = "/api/v1/findings/{id}/ack", tag = "findings", operation_id = "ack_finding",
    params(("id" = Uuid, Path, description = "Finding id")), request_body = FindingActionRequest,
    responses((status = 204, description = "Acknowledged"), (status = 404, description = "Not found / already resolved")))]
pub async fn ack_finding(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<FindingActionRequest>,
) -> axum::response::Response {
    transition(state, principal, id, req, Action::Ack).await
}

#[utoipa::path(post, path = "/api/v1/findings/{id}/resolve", tag = "findings", operation_id = "resolve_finding",
    params(("id" = Uuid, Path, description = "Finding id")), request_body = FindingActionRequest,
    responses((status = 204, description = "Resolved"), (status = 404, description = "Not found / already resolved")))]
pub async fn resolve_finding(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<FindingActionRequest>,
) -> axum::response::Response {
    transition(state, principal, id, req, Action::Resolve).await
}

enum Action {
    Ack,
    Resolve,
}

/// Shared ack/resolve path: load the finding (to find its detection for the
/// grant check), require `edit` on the detection, then apply the lifecycle move.
async fn transition(
    state: AppState,
    principal: Option<Extension<Principal>>,
    id: Uuid,
    req: FindingActionRequest,
    action: Action,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let f = match finding::get(&state.metadata, &tenant, id).await {
        Ok(Some(f)) => f,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DETECTION,
        &f.detection_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let note = req.note.as_deref();
    let result = match action {
        Action::Ack => {
            finding::acknowledge(&state.metadata, &tenant, id, &caller.subject, note).await
        }
        Action::Resolve => finding::resolve(&state.metadata, &tenant, id, note).await,
    };
    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found or already resolved").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
