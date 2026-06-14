//! `GET /api/v1/flows/:id` — one flow in full.

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
use crate::authz::{self, ACTION_VIEW, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/flows/{id}",
    tag = "flows",
    operation_id = "get_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Flow", body = FlowDetail),
        (status = 403, description = "Not allowed to view this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_flow(
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
        ACTION_VIEW,
        KIND_FLOW,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_detail(&rec, &state.flows)).into_response()
}
