//! REST surface. Thin handlers: extract → store → DTO. No auth.
//!
//! Mounts under `/api/*`. The router is generic over a parent `S`
//! state so it composes into the server builder's final assembly.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{sse::Sse, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use utoipa::OpenApi;

use crate::ai_runtime::{AgentRunError, AiRuntime, ProviderStatusDto};
use crate::domain::{
    Agent, AgentSummary, CreateAgent, CreateFlow, DomainError, FirePayload, FireResponse, Flow,
    FlowSummary, Run, UpdateAgent, UpdateFlow,
};
use crate::flow_engine::{FireOutcome, FlowEngine, FlowEngineError};
use crate::sse::{EventHub, FlowEvent, RunEvent};
use crate::store::{AgentStore, FlowStore, RunStore};
use starter_flow_spi::agent_session::AgentSessionStore;
use starter_flow_spi::event_dto::{slot_value_to_json, NodeSlotValue};
use starter_flow_spi::flow::FlowEvent as EngineFlowEvent;
use starter_spi::ai::HistoryMessage;

#[derive(Clone)]
pub struct RestState {
    pub flows: Arc<FlowStore>,
    pub agents: Arc<AgentStore>,
    pub runs: Arc<RunStore>,
    pub hub: Arc<EventHub>,
    pub engine: FlowEngine,
    pub ai: AiRuntime,
    /// MEMORY.md Phase M-D — page-builder persistence. Reused by
    /// the builder stream (to persist turns + tree artifacts) and
    /// by the artifact GET endpoint (zero-token page reload).
    pub agent_sessions: Arc<dyn AgentSessionStore>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_flows, create_flow, get_flow, update_flow, delete_flow,
        fire_flow, list_runs,
        list_agents, create_agent, get_agent, update_agent, delete_agent,
        run_agent, list_providers,
        sidebar_events, flow_events,
        crate::builder_stream::builder_stream,
        create_session, get_latest_artifact,
        list_artifact_versions, get_artifact_version,
    ),
    components(schemas(
        Flow, FlowSummary, CreateFlow, UpdateFlow,
        Agent, AgentSummary, CreateAgent, UpdateAgent,
        Run, FirePayload, FireResponse,
        FlowEvent, RunEvent,
        ProviderStatusDto,
        crate::builder_stream::BuilderRequest,
        CreateSession, SessionCreated, ArtifactDto, ArtifactMetaDto,
    ))
)]
pub struct FlowAgentApi;

pub fn router<S>(state: RestState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // Flows
        .route("/api/flows", get(list_flows).post(create_flow))
        .route(
            "/api/flows/{id}",
            get(get_flow).put(update_flow).delete(delete_flow),
        )
        .route("/api/flows/{id}/fire", post(fire_flow))
        .route("/api/flows/{id}/runs", get(list_runs))
        // Agents
        .route("/api/agents", get(list_agents).post(create_agent))
        .route(
            "/api/agents/{id}",
            get(get_agent).put(update_agent).delete(delete_agent),
        )
        .route("/api/agents/{id}/run", post(run_agent))
        // Providers (Settings page surfaces this read-only).
        .route("/api/providers", get(list_providers))
        // Live page builder (SSE). No compression layer is mounted
        // on this app, so no exclusion is required; the headers
        // applied inside `builder_stream` still defeat proxy-level
        // buffering (Vite dev, nginx).
        .route(
            "/api/builder/stream",
            post(crate::builder_stream::builder_stream),
        )
        // Agent sessions — page-builder persistence (MEMORY.md
        // Phase M-D). The artifact GET endpoints are zero-token
        // page-reload paths; sessions are surface-owned UUIDv7s
        // returned by `POST /api/sessions`.
        .route("/api/sessions", post(create_session))
        .route(
            "/api/sessions/{id}/artifacts/{key}",
            get(get_latest_artifact),
        )
        .route(
            "/api/sessions/{id}/artifacts/{key}/versions",
            get(list_artifact_versions),
        )
        .route(
            "/api/sessions/{id}/artifacts/{key}/versions/{version}",
            get(get_artifact_version),
        )
        // SSE
        .route("/api/events", get(sidebar_events))
        .route("/api/flows/{id}/events", get(flow_events))
        .with_state(state)
}

// ---------------------------------------------------------------------
// Flow handlers
// ---------------------------------------------------------------------

#[utoipa::path(get, path = "/api/flows", tag = "flows",
    responses((status = 200, body = [FlowSummary])))]
async fn list_flows(State(s): State<RestState>) -> Result<Json<Vec<FlowSummary>>, ApiError> {
    Ok(Json(s.flows.list().await?))
}

#[utoipa::path(post, path = "/api/flows", tag = "flows", request_body = CreateFlow,
    responses((status = 201, body = Flow)))]
async fn create_flow(
    State(s): State<RestState>,
    Json(body): Json<CreateFlow>,
) -> Result<(StatusCode, Json<Flow>), ApiError> {
    let flow = s.flows.create(body).await?;
    let _ = s.hub.sidebar.send(FlowEvent::FlowCreated {
        id: flow.id.clone(),
        name: flow.name.clone(),
    });
    Ok((StatusCode::CREATED, Json(flow)))
}

#[utoipa::path(get, path = "/api/flows/{id}", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    responses((status = 200, body = Flow), (status = 404)))]
async fn get_flow(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> Result<Json<Flow>, ApiError> {
    Ok(Json(s.flows.get(&id).await?))
}

#[utoipa::path(put, path = "/api/flows/{id}", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = UpdateFlow,
    responses((status = 200, body = Flow), (status = 404), (status = 409)))]
async fn update_flow(
    State(s): State<RestState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateFlow>,
) -> Result<Json<Flow>, ApiError> {
    let prev = s.flows.get(&id).await.ok();
    let flow = s.flows.update(&id, body).await?;
    if prev.as_ref().map(|p| &p.name) != Some(&flow.name) {
        let _ = s.hub.sidebar.send(FlowEvent::FlowRenamed {
            id: flow.id.clone(),
            name: flow.name.clone(),
        });
    }
    Ok(Json(flow))
}

#[utoipa::path(delete, path = "/api/flows/{id}", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    responses((status = 204), (status = 404)))]
async fn delete_flow(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    s.flows.delete(&id).await?;
    let _ = s.hub.sidebar.send(FlowEvent::FlowDeleted { id });
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/flows/{id}/fire", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    request_body = FirePayload,
    responses(
        (status = 202, body = FireResponse),
        (status = 404),
        (status = 422, description = "Graph is missing a trigger or could not be materialised as a FlowTopology"),
    ))]
async fn fire_flow(
    State(s): State<RestState>,
    Path(id): Path<String>,
    Json(body): Json<FirePayload>,
) -> Result<(StatusCode, Json<FireResponse>), ApiError> {
    // Load the flow first so 404 wins over 422.
    let flow = s.flows.get(&id).await?;

    // Materialise + fire. Any structural / kind error here becomes
    // 422; we have not recorded a run row yet so no cleanup is
    // needed in that path.
    let outcome = s
        .engine
        .fire(&flow.id, &flow.graph, body.payload.clone())
        .await
        .map_err(ApiError::from)?;

    // Now that the engine has accepted the run, persist the host-
    // side run row and emit RunStarted.
    let run = s.runs.record_started(&flow.id).await?;
    let _ = s.hub.runs.send(RunEvent::RunStarted {
        flow_id: flow.id.clone(),
        run_id: run.id.clone(),
    });

    // Spawn the SSE pump task. It owns the RunHandle and translates
    // engine FlowEvents → host RunEvents until terminal, then
    // persists the terminal status via RunStore::record_finished.
    let hub = s.hub.clone();
    let runs_store = s.runs.clone();
    let flow_id_for_task = flow.id.clone();
    let run_db_id = run.id.clone();
    tokio::spawn(async move {
        drive_run(hub, runs_store, flow_id_for_task, run_db_id, outcome).await;
    });

    Ok((StatusCode::ACCEPTED, Json(FireResponse { run_id: run.id })))
}

/// Pump engine `FlowEvent`s onto `EventHub.runs` as `RunEvent`s and
/// persist the terminal status to the host's `runs` table. Owns the
/// `RunHandle` for the lifetime of the run.
async fn drive_run(
    hub: Arc<EventHub>,
    runs_store: Arc<RunStore>,
    flow_id: String,
    run_db_id: String,
    mut outcome: FireOutcome,
) {
    use tokio::sync::broadcast::error::RecvError;

    let mut terminal_status = "error".to_owned();
    let mut terminal_trace: Option<serde_json::Value> = None;
    let mut rx = std::mem::replace(
        &mut outcome.handle.initial_rx,
        outcome.handle.events_tx.subscribe(),
    );

    loop {
        match rx.recv().await {
            Ok(ev) => {
                let done = handle_engine_event(&hub, &flow_id, &run_db_id, &outcome, &ev);
                if let Some((status, trace)) = done {
                    terminal_status = status;
                    terminal_trace = trace;
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }

    // Also await the coordinator task so the engine cleans up. Its
    // return value is the RunStatus mirror of what we observed above.
    let _ = outcome.handle.join.await;

    if let Err(e) = runs_store
        .record_finished(&run_db_id, &terminal_status, terminal_trace.as_ref())
        .await
    {
        tracing::error!(error = %e, run_id = %run_db_id, "failed to persist terminal run status");
    }

    let _ = hub.runs.send(RunEvent::RunFinished {
        flow_id,
        run_id: run_db_id,
        status: terminal_status,
    });
}

/// Translate a single engine `FlowEvent` into zero-or-more host
/// `RunEvent`s. Returns `Some((status, trace))` if the event is
/// terminal (the caller breaks the loop).
fn handle_engine_event(
    hub: &EventHub,
    flow_id: &str,
    run_db_id: &str,
    outcome: &FireOutcome,
    ev: &EngineFlowEvent,
) -> Option<(String, Option<serde_json::Value>)> {
    match ev {
        EngineFlowEvent::RunStarted { .. } => {
            // Already emitted from the handler — nothing to do.
            None
        }
        EngineFlowEvent::NodeStarted { node, .. } => {
            if let Some(ui_id) = outcome.ui_node_id(node) {
                let _ = hub.runs.send(RunEvent::NodeStatus {
                    flow_id: flow_id.to_owned(),
                    run_id: run_db_id.to_owned(),
                    node_id: ui_id.to_owned(),
                    status: "running".into(),
                });
            }
            None
        }
        EngineFlowEvent::NodeEmitted { node, slot, .. } => {
            // EdgeActive for every UI edge fanning out from this slot.
            if let Some(edges) = outcome.edge_index.get(&(node.clone(), slot.clone())) {
                for edge_id in edges {
                    let _ = hub.runs.send(RunEvent::EdgeActive {
                        flow_id: flow_id.to_owned(),
                        run_id: run_db_id.to_owned(),
                        edge_id: edge_id.clone(),
                    });
                }
            }
            // Forward the slot value as a NodeOutput event so the
            // frontend (and any curl-based listener) can see what the
            // node actually produced. Uses the shared
            // `NodeSlotValue::from_event` projection so every host
            // emits identically shaped values.
            if let Some(ui_id) = outcome.ui_node_id(node) {
                if let Some(dto) = NodeSlotValue::from_event(ev) {
                    let _ = hub.runs.send(RunEvent::NodeOutput {
                        flow_id: flow_id.to_owned(),
                        run_id: run_db_id.to_owned(),
                        node_id: ui_id.to_owned(),
                        slot: dto.slot,
                        value: dto.value,
                    });
                }
                // Treat an emit as the node being "ok" for now — the
                // engine has no NodeCompleted variant and most kinds
                // emit exactly one terminal output.
                let _ = hub.runs.send(RunEvent::NodeStatus {
                    flow_id: flow_id.to_owned(),
                    run_id: run_db_id.to_owned(),
                    node_id: ui_id.to_owned(),
                    status: "ok".into(),
                });
            }
            None
        }
        EngineFlowEvent::NodeFailed { node, error, .. } => {
            if let Some(ui_id) = outcome.ui_node_id(node) {
                let _ = hub.runs.send(RunEvent::NodeStatus {
                    flow_id: flow_id.to_owned(),
                    run_id: run_db_id.to_owned(),
                    node_id: ui_id.to_owned(),
                    status: "error".into(),
                });
            }
            tracing::warn!(node = %node, error = %error, "flow node failed");
            None
        }
        EngineFlowEvent::RunCompleted { output, .. } => {
            let trace = serde_json::to_value(
                output
                    .iter()
                    .map(|(k, v)| (k.clone(), slot_value_to_json(v)))
                    .collect::<serde_json::Map<_, _>>(),
            )
            .ok();
            Some(("ok".to_owned(), trace))
        }
        EngineFlowEvent::RunFailed { error, .. } => Some((
            "error".to_owned(),
            Some(serde_json::json!({ "error": error })),
        )),
        EngineFlowEvent::RunCancelled { .. } => Some(("cancelled".to_owned(), None)),
        _ => None,
    }
}

#[utoipa::path(get, path = "/api/flows/{id}/runs", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    responses((status = 200, body = [Run])))]
async fn list_runs(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Run>>, ApiError> {
    Ok(Json(s.runs.list_for_flow(&id).await?))
}

// ---------------------------------------------------------------------
// Agent handlers
// ---------------------------------------------------------------------

#[utoipa::path(get, path = "/api/agents", tag = "agents",
    responses((status = 200, body = [AgentSummary])))]
async fn list_agents(State(s): State<RestState>) -> Result<Json<Vec<AgentSummary>>, ApiError> {
    Ok(Json(s.agents.list().await?))
}

#[utoipa::path(post, path = "/api/agents", tag = "agents", request_body = CreateAgent,
    responses((status = 201, body = Agent)))]
async fn create_agent(
    State(s): State<RestState>,
    Json(body): Json<CreateAgent>,
) -> Result<(StatusCode, Json<Agent>), ApiError> {
    let agent = s.agents.create(body).await?;
    let _ = s.hub.sidebar.send(FlowEvent::AgentCreated {
        id: agent.id.clone(),
        name: agent.name.clone(),
    });
    Ok((StatusCode::CREATED, Json(agent)))
}

#[utoipa::path(get, path = "/api/agents/{id}", tag = "agents",
    params(("id" = String, Path, description = "Agent id")),
    responses((status = 200, body = Agent), (status = 404)))]
async fn get_agent(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> Result<Json<Agent>, ApiError> {
    Ok(Json(s.agents.get(&id).await?))
}

#[utoipa::path(put, path = "/api/agents/{id}", tag = "agents",
    params(("id" = String, Path, description = "Agent id")),
    request_body = UpdateAgent,
    responses((status = 200, body = Agent), (status = 404)))]
async fn update_agent(
    State(s): State<RestState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAgent>,
) -> Result<Json<Agent>, ApiError> {
    let prev = s.agents.get(&id).await.ok();
    let agent = s.agents.update(&id, body).await?;
    if prev.as_ref().map(|p| &p.name) != Some(&agent.name) {
        let _ = s.hub.sidebar.send(FlowEvent::AgentRenamed {
            id: agent.id.clone(),
            name: agent.name.clone(),
        });
    }
    Ok(Json(agent))
}

#[utoipa::path(delete, path = "/api/agents/{id}", tag = "agents",
    params(("id" = String, Path, description = "Agent id")),
    responses((status = 204), (status = 404)))]
async fn delete_agent(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    s.agents.delete(&id).await?;
    let _ = s.hub.sidebar.send(FlowEvent::AgentDeleted { id });
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------
// Agent chat (SSE response)
// ---------------------------------------------------------------------

/// Body shape sent by `@nube/starter-ui-chat`'s default `createSseAdapter`.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct AgentRunRequest {
    /// New user input — text + optional metadata (we only consume `text`).
    pub input: ChatSendInputDto,
    /// Prior conversation turns the chat surface has accumulated.
    #[serde(default)]
    pub history: Vec<ChatMessageDto>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ChatSendInputDto {
    pub text: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: String,
}

#[utoipa::path(post, path = "/api/agents/{id}/run", tag = "agents",
    params(("id" = String, Path, description = "Agent id")),
    request_body = AgentRunRequest,
    responses(
        (status = 200, description = "SSE chat stream", content_type = "text/event-stream"),
        (status = 404),
        (status = 422, description = "Provider unknown / unavailable"),
    ))]
async fn run_agent(
    State(s): State<RestState>,
    Path(id): Path<String>,
    Json(body): Json<AgentRunRequest>,
) -> Result<axum::response::Response, ApiError> {
    let agent = s.agents.get(&id).await?;
    let history: Vec<HistoryMessage> = body
        .history
        .into_iter()
        .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "system"))
        .map(|m| HistoryMessage {
            role: m.role,
            content: m.content,
        })
        .collect();

    let stream = s
        .ai
        .run_agent(&agent, body.input.text, history)
        .map_err(ApiError::from)?;

    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    );
    Ok(sse.into_response())
}

#[utoipa::path(get, path = "/api/providers", tag = "providers",
    responses((status = 200, body = [ProviderStatusDto])))]
async fn list_providers(
    State(s): State<RestState>,
) -> Result<Json<Vec<ProviderStatusDto>>, ApiError> {
    Ok(Json(s.ai.list_providers().await))
}

// ---------------------------------------------------------------------
// SSE handlers
// ---------------------------------------------------------------------

#[utoipa::path(get, path = "/api/events", tag = "events",
    responses((status = 200, description = "SSE stream", content_type = "text/event-stream")))]
async fn sidebar_events(State(s): State<RestState>) -> impl IntoResponse {
    let rx = s.hub.sidebar.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|r| async move { r.ok() });
    starter_server::sse::from_stream(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

#[utoipa::path(get, path = "/api/flows/{id}/events", tag = "flows",
    params(("id" = String, Path, description = "Flow id")),
    responses((status = 200, description = "SSE run events", content_type = "text/event-stream")))]
async fn flow_events(
    State(s): State<RestState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let rx = s.hub.runs.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |r| {
        let want = id.clone();
        async move {
            let ev = r.ok()?;
            let belongs = match &ev {
                RunEvent::RunStarted { flow_id, .. }
                | RunEvent::NodeStatus { flow_id, .. }
                | RunEvent::EdgeActive { flow_id, .. }
                | RunEvent::NodeOutput { flow_id, .. }
                | RunEvent::RunFinished { flow_id, .. } => flow_id == &want,
            };
            belongs.then_some(ev)
        }
    });
    Sse::new(stream.map(|ev| {
        let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
        Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(json))
    }))
    .keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ---------------------------------------------------------------------
// Agent sessions (MEMORY.md Phase M-D)
//
// `POST /api/sessions` — create a session up front so the surface
// can echo the id back to the user. Optional; the builder route
// also creates one implicitly when a request arrives with no
// `session_id` (and persistence enabled).
//
// `GET  /api/sessions/{id}/artifacts/{key}` — zero-token page-reload
// path. Frontend on mount fetches the latest tree to render the
// canvas without spending model budget. (MEMORY.md M4.)
//
// `GET  /api/sessions/{id}/artifacts/{key}/versions` — undo/version
// picker; lists metadata only (cheap), bodies live behind the
// version-specific endpoint below.
//
// `GET  /api/sessions/{id}/artifacts/{key}/versions/{version}` —
// fetch a specific historical body (undo target).
// ---------------------------------------------------------------------

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CreateSession {
    /// Surface-defined kind (`"page-builder"`, `"chat"`, ...).
    /// Defaults to `"page-builder"` to fit this example's UX.
    #[serde(default = "default_session_kind")]
    pub kind: String,
    /// Optional principal subject. Defaults to `"system"` —
    /// flow-agent has no auth (SCOPE F4), so every session is
    /// effectively unowned per MEMORY.md "Decisions made".
    #[serde(default)]
    pub owner: Option<String>,
}

fn default_session_kind() -> String {
    "page-builder".to_owned()
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SessionCreated {
    pub session_id: String,
}

#[utoipa::path(post, path = "/api/sessions", tag = "sessions",
    request_body = CreateSession,
    responses(
        (status = 201, description = "Session created", body = SessionCreated),
    ))]
async fn create_session(
    State(s): State<RestState>,
    Json(req): Json<CreateSession>,
) -> Result<(StatusCode, Json<SessionCreated>), ApiError> {
    use starter_flow_spi::agent_session::AgentSessionId;
    let id = AgentSessionId::new();
    let owner = req.owner.unwrap_or_else(|| "system".to_owned());
    s.agent_sessions
        .create(id, &req.kind, &owner, serde_json::json!({}))
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(SessionCreated {
            session_id: id.to_string(),
        }),
    ))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ArtifactDto {
    pub session_id: String,
    pub key: String,
    pub version: u32,
    pub parent_version: Option<u32>,
    pub value: serde_json::Value,
    pub value_bytes: u32,
    pub produced_by_seq: Option<u32>,
    pub updated_at: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ArtifactMetaDto {
    pub version: u32,
    pub parent_version: Option<u32>,
    pub value_bytes: u32,
    pub produced_by_seq: Option<u32>,
    pub updated_at: String,
}

fn parse_session_id(raw: &str) -> Result<starter_flow_spi::agent_session::AgentSessionId, ApiError>
{
    starter_flow_spi::agent_session::AgentSessionId::parse(raw)
        .map_err(|_| ApiError::Domain(DomainError::Invalid("invalid session_id".into())))
}

#[utoipa::path(get, path = "/api/sessions/{id}/artifacts/{key}", tag = "sessions",
    params(
        ("id" = String, Path, description = "Session id (UUIDv7 string)"),
        ("key" = String, Path, description = "Artifact key (e.g. \"tree\")"),
    ),
    responses(
        (status = 200, description = "Latest artifact body", body = ArtifactDto),
        (status = 404, description = "No artifact for this key"),
    ))]
async fn get_latest_artifact(
    State(s): State<RestState>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<ArtifactDto>, ApiError> {
    let session_id = parse_session_id(&id)?;
    let artifact = s
        .agent_sessions
        .latest_artifact(session_id, &key)
        .await?
        .ok_or_else(|| ApiError::Domain(DomainError::NotFound(format!("artifact {key}"))))?;
    Ok(Json(artifact_to_dto(artifact)))
}

#[utoipa::path(get, path = "/api/sessions/{id}/artifacts/{key}/versions", tag = "sessions",
    params(
        ("id" = String, Path, description = "Session id"),
        ("key" = String, Path, description = "Artifact key"),
    ),
    responses(
        (status = 200, description = "Every version newest first", body = [ArtifactMetaDto]),
    ))]
async fn list_artifact_versions(
    State(s): State<RestState>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<Vec<ArtifactMetaDto>>, ApiError> {
    let session_id = parse_session_id(&id)?;
    let metas = s
        .agent_sessions
        .list_artifact_versions(session_id, &key)
        .await?;
    Ok(Json(metas.into_iter().map(meta_to_dto).collect()))
}

#[utoipa::path(get, path = "/api/sessions/{id}/artifacts/{key}/versions/{version}",
    tag = "sessions",
    params(
        ("id" = String, Path, description = "Session id"),
        ("key" = String, Path, description = "Artifact key"),
        ("version" = u32, Path, description = "Artifact version"),
    ),
    responses(
        (status = 200, description = "Historical artifact body", body = ArtifactDto),
        (status = 404, description = "No such version"),
    ))]
async fn get_artifact_version(
    State(s): State<RestState>,
    Path((id, key, version)): Path<(String, String, u32)>,
) -> Result<Json<ArtifactDto>, ApiError> {
    let session_id = parse_session_id(&id)?;
    let artifact = s
        .agent_sessions
        .artifact_at(session_id, &key, version)
        .await?
        .ok_or_else(|| {
            ApiError::Domain(DomainError::NotFound(format!("{key} v{version}")))
        })?;
    Ok(Json(artifact_to_dto(artifact)))
}

fn artifact_to_dto(a: starter_flow_spi::agent_session::Artifact) -> ArtifactDto {
    ArtifactDto {
        session_id: a.session_id.to_string(),
        key: a.key,
        version: a.version,
        parent_version: a.parent_version,
        value: a.value,
        value_bytes: a.value_bytes,
        produced_by_seq: a.produced_by_seq,
        updated_at: a.updated_at.to_rfc3339(),
    }
}

fn meta_to_dto(m: starter_flow_spi::agent_session::ArtifactMeta) -> ArtifactMetaDto {
    ArtifactMetaDto {
        version: m.version,
        parent_version: m.parent_version,
        value_bytes: m.value_bytes,
        produced_by_seq: m.produced_by_seq,
        updated_at: m.updated_at.to_rfc3339(),
    }
}

// ---------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------

pub enum ApiError {
    Domain(DomainError),
    Engine(FlowEngineError),
    Agent(AgentRunError),
    /// MEMORY.md Phase M-D — agent-session store surfaced an
    /// error (size cap, backend, missing session). Mapped to
    /// 4xx/5xx by [`ApiError::into_response`].
    AgentSession(starter_flow_spi::agent_session::AgentSessionError),
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

impl From<FlowEngineError> for ApiError {
    fn from(e: FlowEngineError) -> Self {
        Self::Engine(e)
    }
}

impl From<AgentRunError> for ApiError {
    fn from(e: AgentRunError) -> Self {
        Self::Agent(e)
    }
}

impl From<starter_flow_spi::agent_session::AgentSessionError> for ApiError {
    fn from(e: starter_flow_spi::agent_session::AgentSessionError) -> Self {
        Self::AgentSession(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            ApiError::Domain(e) => match e {
                DomainError::NotFound(_) => (StatusCode::NOT_FOUND, e.to_string()),
                DomainError::VersionConflict => (StatusCode::CONFLICT, e.to_string()),
                DomainError::Invalid(_) => (StatusCode::BAD_REQUEST, e.to_string()),
                DomainError::Db(_) | DomainError::Json(_) => {
                    tracing::error!(error = %e, "internal error");
                    (StatusCode::INTERNAL_SERVER_ERROR, "internal".into())
                }
            },
            ApiError::Agent(e) => match e {
                AgentRunError::UnknownProvider(_) | AgentRunError::ProviderUnavailable(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
                }
                AgentRunError::Registry(_) => {
                    tracing::error!(error = %e, "flow registry error during agent run");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            },
            ApiError::Engine(e) => match e {
                FlowEngineError::Parse(_) | FlowEngineError::Invalid(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
                }
                FlowEngineError::Engine(_) | FlowEngineError::Fire(_) => {
                    tracing::error!(error = %e, "engine error");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            },
            ApiError::AgentSession(e) => {
                use starter_flow_spi::agent_session::AgentSessionError;
                match e {
                    AgentSessionError::SessionNotFound(_) => {
                        (StatusCode::NOT_FOUND, e.to_string())
                    }
                    AgentSessionError::TurnTooLarge { .. }
                    | AgentSessionError::ArtifactTooLarge { .. } => {
                        // M8 / M12 cap surfaces — payload-too-large
                        // is the precise status; clients shrink the
                        // body and retry.
                        (StatusCode::PAYLOAD_TOO_LARGE, e.to_string())
                    }
                    AgentSessionError::Backend(_) => {
                        tracing::error!(error = %e, "agent-session backend error");
                        (StatusCode::INTERNAL_SERVER_ERROR, "internal".into())
                    }
                    // `#[non_exhaustive]` future variants — surface
                    // as 500 with the Display impl; refine when added.
                    _ => {
                        tracing::error!(error = %e, "unhandled agent-session error");
                        (StatusCode::INTERNAL_SERVER_ERROR, "internal".into())
                    }
                }
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
