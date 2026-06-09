//! `GET /api/v1/agents` — the caller's agents, filtered to viewable ones.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::agent::AgentSummary;
use nexus_store::agent;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::authz::{self, ACTION_VIEW, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/agents",
    tag = "agents",
    operation_id = "list_agents",
    responses((status = 200, description = "Agents", body = [AgentSummary])),
)]
pub async fn list_agents(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let recs = match agent::list(&state.metadata, &tenant).await {
        Ok(r) => r,
        Err(e) => return IntoResponse(e).into_response(),
    };
    // Drop agents the caller may not view, mirroring the dashboards list gate.
    let mut out = Vec::with_capacity(recs.len());
    for rec in &recs {
        if authz::can(
            state.engine.as_ref(),
            caller,
            ACTION_VIEW,
            KIND_AGENT,
            &rec.id.to_string(),
            &tenant,
        )
        .await
        {
            out.push(to_summary(rec));
        }
    }
    Json(out).into_response()
}
