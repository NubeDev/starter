//! `POST /api/v1/flows` — define a flow for the caller's tenant.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::{CreateFlowRequest, FlowDetail};
use nexus_store::flow::{self, NewFlow};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/flows",
    tag = "flows",
    operation_id = "create_flow",
    request_body = CreateFlowRequest,
    responses((status = 200, description = "Created", body = FlowDetail)),
)]
pub async fn create_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateFlowRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewFlow {
        name: req.name,
        input: req.input,
        pipeline: req.pipeline.unwrap_or_else(|| json!([])),
        output: req.output,
        enabled: req.enabled.unwrap_or(false),
    };
    match flow::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec, &state.flows)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
