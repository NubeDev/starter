//! `GET /api/v1/insights/:id` — fetch one stored insight.
//!
//! LAYER: transport (REST). Resolve → authorize → shape DTO → return.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::insight::InsightSummary;
use nexus_store::insight;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_summary;
use crate::authz::{self, ACTION_VIEW, KIND_INSIGHT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/insights/{id}",
    tag = "insights",
    operation_id = "get_insight",
    params(("id" = Uuid, Path, description = "Insight id")),
    responses(
        (status = 200, description = "The insight", body = InsightSummary),
        (status = 403, description = "Not authorized to view this insight"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_insight(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match insight::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_VIEW,
        KIND_INSIGHT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_summary(&rec)).into_response()
}
