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

use crate::domain::{
    Agent, AgentSummary, CreateAgent, CreateFlow, DomainError, FirePayload, FireResponse, Flow,
    FlowSummary, Run, UpdateAgent, UpdateFlow,
};
use crate::flow_engine::{FireOutcome, FlowEngine, FlowEngineError};
use crate::sse::{EventHub, FlowEvent, RunEvent};
use crate::store::{AgentStore, FlowStore, RunStore};
use starter_flow_spi::flow::FlowEvent as EngineFlowEvent;

#[derive(Clone)]
pub struct RestState {
    pub flows: Arc<FlowStore>,
    pub agents: Arc<AgentStore>,
    pub runs: Arc<RunStore>,
    pub hub: Arc<EventHub>,
    pub engine: FlowEngine,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_flows, create_flow, get_flow, update_flow, delete_flow,
        fire_flow, list_runs,
        list_agents, create_agent, get_agent, update_agent, delete_agent,
        sidebar_events, flow_events,
    ),
    components(schemas(
        Flow, FlowSummary, CreateFlow, UpdateFlow,
        Agent, AgentSummary, CreateAgent, UpdateAgent,
        Run, FirePayload, FireResponse,
        FlowEvent, RunEvent,
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
            // Treat an emit as the node being "ok" for now — the
            // engine has no NodeCompleted variant and most kinds emit
            // exactly one terminal output.
            if let Some(ui_id) = outcome.ui_node_id(node) {
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
        EngineFlowEvent::RunCompleted { .. } => Some(("ok".to_owned(), None)),
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
// Error mapping
// ---------------------------------------------------------------------

pub enum ApiError {
    Domain(DomainError),
    Engine(FlowEngineError),
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
            ApiError::Engine(e) => match e {
                FlowEngineError::Parse(_) | FlowEngineError::Invalid(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
                }
                FlowEngineError::Engine(_) | FlowEngineError::Fire(_) => {
                    tracing::error!(error = %e, "engine error");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            },
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
