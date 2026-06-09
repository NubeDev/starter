//! `POST /api/v1/agents/:id/sessions` — open and start a session against an
//! agent. Persists a session row, kicks off the streaming run on the
//! SessionRunner, and returns the session id plus a signed SSE token.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_ai::ModelRef;
use nexus_spi::dto::agent::{CreateSessionRequest, CreateSessionResponse};
use nexus_store::agent::{self, NewSession};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_EDIT, KIND_AGENT};
use crate::middleware::tenant::caller;
use crate::middleware::StreamClaims;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/agents/{id}/sessions",
    tag = "agents",
    operation_id = "create_agent_session",
    params(("id" = Uuid, Path, description = "Agent id")),
    request_body = CreateSessionRequest,
    responses(
        (status = 200, description = "Session started", body = CreateSessionResponse),
        (status = 403, description = "Not allowed to run this agent"),
        (status = 404, description = "Agent not found in this tenant"),
    ),
)]
pub async fn create_agent_session(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<CreateSessionRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // The agent must exist and be runnable (edit grant) by the caller.
    let agent_rec = match agent::get(&state.metadata, &tenant, agent_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_AGENT,
        &agent_id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    // Persist the opening turn as a pending session.
    let new = NewSession {
        agent_id,
        transcript: json!([{ "role": "user", "content": req.prompt }]),
    };
    let session = match agent::insert_session(&state.metadata, &tenant, &new).await {
        Ok(s) => s,
        Err(e) => return IntoResponse(e).into_response(),
    };

    // Mark running and kick off the streaming run.
    let _ = agent::set_session_status(&state.metadata, &tenant, session.id, "running").await;
    let model = parse_model(&agent_rec.model);
    let inputs = crate::agents::PromptInputs::from_config(&agent_rec.config);
    state.sessions.start(crate::agents::SessionRun {
        metadata: state.metadata.clone(),
        tenant: tenant.clone(),
        session_id: session.id,
        model,
        system_prompt: agent_rec.system_prompt.clone(),
        inputs,
        prompt: req.prompt,
    });

    // Mint a short-lived SSE token bound to this session id. We reuse the stream
    // signer; datasource_id is unused for agent sessions.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let claims = StreamClaims {
        stream_id: session.id.to_string(),
        datasource_id: String::new(),
        tenant_id: tenant.clone(),
        permission: ACTION_EDIT.to_string(),
        exp: now + state.stream_token_ttl.as_secs(),
    };
    let token = state.stream_signer.mint(&claims);

    Json(CreateSessionResponse {
        id: session.id,
        status: "running".to_string(),
        token,
    })
    .into_response()
}

/// Interpret the stored model string: a known size alias maps to that tier,
/// anything else is a concrete provider id passed through to the facade.
fn parse_model(model: &str) -> ModelRef {
    match model {
        "small" => ModelRef::small(),
        "medium" => ModelRef::medium(),
        "large" => ModelRef::large(),
        other => ModelRef::concrete(other),
    }
}
