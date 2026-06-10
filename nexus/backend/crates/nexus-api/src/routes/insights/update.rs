//! `PATCH /api/v1/insights/:id` — rename or rewrite a stored insight.
//!
//! LAYER: transport (REST). Resolve → authorize → validate → call domain → return.
//! Authorized as `edit` on the insight's immutable id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::insight::{InsightSummary, UpdateInsightRequest};
use nexus_store::insight::{self, InsightPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_summary;
use super::validate::compiles;
use crate::authz::{self, ACTION_EDIT, KIND_INSIGHT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/insights/{id}",
    tag = "insights",
    operation_id = "update_insight",
    params(("id" = Uuid, Path, description = "Insight id")),
    request_body = UpdateInsightRequest,
    responses(
        (status = 200, description = "Updated", body = InsightSummary),
        (status = 400, description = "The new script does not compile"),
        (status = 403, description = "Not authorized to edit this insight"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_insight(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateInsightRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_EDIT,
        KIND_INSIGHT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    // A replacement script must still compile; reject before persisting.
    if let Some(script) = &req.script {
        if let Err(msg) = compiles(script) {
            return (StatusCode::BAD_REQUEST, msg).into_response();
        }
    }
    let patch = InsightPatch {
        name: req.name,
        script: req.script,
        params_schema: req.params_schema,
    };
    match insight::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => Json(to_summary(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
