//! `GET /api/v1/agents/:id` — one agent in full.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::agent::AgentDetail;
use nexus_store::agent;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}",
    tag = "agents",
    operation_id = "get_agent",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses(
        (status = 200, description = "Agent", body = AgentDetail),
        (status = 403, description = "Not allowed to view this agent"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_agent(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match agent::get(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_AGENT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_detail(&rec)).into_response()
}
