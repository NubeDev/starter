//! `POST /api/v1/flows/:id/start` — run a saved flow.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::FlowDetail;
use nexus_store::flow;
use serde_json::Value;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_EDIT, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/flows/{id}/start",
    tag = "flows",
    operation_id = "start_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Running", body = FlowDetail),
        (status = 400, description = "Flow config is invalid"),
        (status = 403, description = "Not allowed to run this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn start_flow(
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

    // The stored pipeline is a JSON array of processor configs.
    let processors: Vec<Value> = rec.pipeline.as_array().cloned().unwrap_or_default();
    // A `datasource`-typed output names a datasource by id; resolve it to the
    // engine's connection material (audited decrypt) before the pipeline builds.
    // Any other output passes through unchanged.
    let output = match crate::datasource_kinds::resolve_flow_output(
        &state,
        &tenant,
        &caller.subject,
        rec.output.clone(),
    )
    .await
    {
        Ok(o) => o,
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(e) = state
        .flows
        .start(&id.to_string(), rec.input.clone(), processors, output)
    {
        return (StatusCode::BAD_REQUEST, e).into_response();
    }
    // Persist the intent so the flow is known-enabled for a future resume.
    if let Err(e) = flow::set_enabled(&state.metadata, &tenant, id, true).await {
        return IntoResponse(e).into_response();
    }
    let started = flow::get(&state.metadata, &tenant, id)
        .await
        .ok()
        .flatten()
        .unwrap_or(rec);
    Json(to_detail(&started, &state.flows)).into_response()
}
