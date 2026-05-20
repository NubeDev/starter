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
use crate::sse::{EventHub, FlowEvent, RunEvent};
use crate::store::{AgentStore, FlowStore, RunStore};

#[derive(Clone)]
pub struct RestState {
    pub flows: Arc<FlowStore>,
    pub agents: Arc<AgentStore>,
    pub runs: Arc<RunStore>,
    pub hub: Arc<EventHub>,
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
    responses((status = 202, body = FireResponse), (status = 404)))]
async fn fire_flow(
    State(s): State<RestState>,
    Path(id): Path<String>,
    Json(_body): Json<FirePayload>,
) -> Result<(StatusCode, Json<FireResponse>), ApiError> {
    // Phase 1: stub. Records a run row but does not actually fire the
    // engine. Phase 3 wires `starter-flow` here.
    let _flow = s.flows.get(&id).await?;
    let run = s.runs.record_started(&id).await?;
    let _ = s.hub.runs.send(RunEvent::RunStarted {
        flow_id: id.clone(),
        run_id: run.id.clone(),
    });
    Ok((StatusCode::ACCEPTED, Json(FireResponse { run_id: run.id })))
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

pub struct ApiError(DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self.0 {
            DomainError::NotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            DomainError::VersionConflict => (StatusCode::CONFLICT, self.0.to_string()),
            DomainError::Invalid(_) => (StatusCode::BAD_REQUEST, self.0.to_string()),
            DomainError::Db(_) | DomainError::Json(_) => {
                tracing::error!(error = %self.0, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal".into())
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
