//! `PUT /api/v1/flows/:id` — edit a flow's name or config.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::UpdateFlowRequest;
use nexus_store::flow::{self, FlowPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_EDIT, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    put,
    path = "/api/v1/flows/{id}",
    tag = "flows",
    operation_id = "update_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    request_body = UpdateFlowRequest,
    responses(
        (status = 204, description = "Updated"),
        (status = 403, description = "Not allowed to edit this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFlowRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
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
    let patch = FlowPatch {
        name: req.name,
        input: req.input,
        pipeline: req.pipeline,
        output: req.output,
        enabled: req.enabled,
    };
    match flow::update(&state.metadata, &tenant, id, &patch).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
