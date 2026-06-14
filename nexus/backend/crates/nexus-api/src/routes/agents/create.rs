//! `POST /api/v1/agents` — define an agent for the caller's tenant.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::agent::{AgentDetail, CreateAgentRequest};
use nexus_store::agent::{self, NewAgent};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/agents",
    tag = "agents",
    operation_id = "create_agent",
    request_body = CreateAgentRequest,
    responses((status = 200, description = "Created", body = AgentDetail)),
)]
pub async fn create_agent(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateAgentRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewAgent {
        name: req.name,
        backend: req.backend,
        model: req.model.unwrap_or_else(|| "large".to_string()),
        system_prompt: req.system_prompt,
        config: req.config.unwrap_or_else(|| json!({})),
    };
    match agent::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
