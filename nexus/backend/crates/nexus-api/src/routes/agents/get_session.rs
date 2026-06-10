//! `GET /api/v1/agents/sessions/:id` — one session in full, including its
//! persisted transcript. Gated by view on the session's parent agent.

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
    path = "/api/v1/agents/sessions/{id}",
    tag = "agents",
    operation_id = "get_agent_session",
    params(("id" = Uuid, Path, description = "Session id")),
    responses(
        (status = 200, description = "Session", body = SessionDetail),
        (status = 403, description = "Not allowed to view this session"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_agent_session(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let session = match agent::get_session(&state.metadata, &tenant, id).await {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    // A session inherits its agent's grants.
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_AGENT,
        &session.agent_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_session(&session)).into_response()
}
