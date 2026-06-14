//! `GET /api/v1/agents/:id/sessions` — sessions for one agent.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::agent::SessionDetail;
use nexus_store::agent;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_session;
use crate::authz::{self, ACTION_VIEW, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/agents/{id}/sessions",
    tag = "agents",
    operation_id = "list_agent_sessions",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses(
        (status = 200, description = "Sessions", body = [SessionDetail]),
        (status = 403, description = "Not allowed to view this agent"),
        (status = 404, description = "Agent not found in this tenant"),
    ),
)]
pub async fn list_agent_sessions(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(agent_id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Ok(None) = agent::get(&state.metadata, &tenant, agent_id).await {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_AGENT,
        &agent_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match agent::list_sessions(&state.metadata, &tenant, agent_id).await {
        Ok(recs) => Json(recs.iter().map(to_session).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
