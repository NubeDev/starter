//! `POST /api/v1/flows/:id/stop` — halt a running flow.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::FlowDetail;
use nexus_store::flow;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_EDIT, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/flows/{id}/stop",
    tag = "flows",
    operation_id = "stop_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Stopped", body = FlowDetail),
        (status = 403, description = "Not allowed to stop this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn stop_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match flow::get(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_FLOW,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    state.flows.stop(&id.to_string());
    if let Err(e) = flow::set_enabled(&state.metadata, &tenant, id, false).await {
        return IntoResponse(e).into_response();
    }
    let stopped = flow::get(&state.metadata, &tenant, id)
        .await
        .ok()
        .flatten()
        .unwrap_or(rec);
    Json(to_detail(&stopped, &state.flows)).into_response()
}
