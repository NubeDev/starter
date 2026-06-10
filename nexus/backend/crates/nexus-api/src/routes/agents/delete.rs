//! `DELETE /api/v1/agents/:id` — remove an agent and its sessions.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::agent;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/agents/{id}",
    tag = "agents",
    operation_id = "delete_agent",
    params(("id" = Uuid, Path, description = "Agent id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not allowed to delete this agent"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_agent(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Ok(None) = agent::get(&state.metadata, &tenant, id).await {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_AGENT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match agent::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
