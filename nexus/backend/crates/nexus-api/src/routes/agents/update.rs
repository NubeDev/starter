//! `PUT /api/v1/agents/:id` — edit an agent's config.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::agent::{AgentDetail, UpdateAgentRequest};
use nexus_store::agent::{self, AgentPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_EDIT, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    put,
    path = "/api/v1/agents/{id}",
    tag = "agents",
    operation_id = "update_agent",
    params(("id" = Uuid, Path, description = "Agent id")),
    request_body = UpdateAgentRequest,
    responses(
        (status = 200, description = "Updated", body = AgentDetail),
        (status = 403, description = "Not allowed to edit this agent"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_agent(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAgentRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Must already exist (and be visible) before the edit gate, so a 404 isn't
    // masked as a 403.
    if let Ok(None) = agent::get(&state.metadata, &tenant, id).await {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_AGENT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let patch = AgentPatch {
        name: req.name,
        backend: req.backend,
        model: req.model,
        // A present `system_prompt` sets it; clearing is not expressed by this
        // verb (an empty string sets an empty prompt), so map to Some(Some/_).
        system_prompt: req.system_prompt.map(Some),
        config: req.config,
    };
    match agent::update(&state.metadata, &tenant, id, &patch).await {
        Ok(true) => match agent::get(&state.metadata, &tenant, id).await {
            Ok(Some(rec)) => Json(to_detail(&rec)).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
            Err(e) => IntoResponse(e).into_response(),
        },
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
