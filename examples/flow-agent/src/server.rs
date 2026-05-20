//! Compose the axum router. No auth: every route is open. The example
//! binds to 127.0.0.1 by default and trusts the OS-level boundary
//! (per SCOPE F4).

use std::sync::Arc;

use axum::Router;
use prometheus::Registry;
use starter_observability::metrics::StandardMetrics;
use starter_server::ServerBuilder;
use starter_store_sqlite::Pool;
use utoipa::OpenApi;

use crate::ai_runtime::AiRuntime;
use crate::flow_engine::FlowEngine;
use crate::rest::{router as rest_router, FlowAgentApi, RestState};
use crate::sse::EventHub;
use crate::store::{AgentStore, FlowStore, RunStore};

#[derive(Clone)]
pub struct AppState;

pub struct Built {
    pub router: Router,
    pub flows: Arc<FlowStore>,
    pub agents: Arc<AgentStore>,
    pub runs: Arc<RunStore>,
    pub hub: Arc<EventHub>,
}

pub fn build(pool: Pool, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Built {
    let sqlx = pool.sqlx().clone();
    let flows = Arc::new(FlowStore::new(sqlx.clone()));
    let agents = Arc::new(AgentStore::new(sqlx.clone()));
    let runs = Arc::new(RunStore::new(sqlx));
    let hub = Arc::new(EventHub::new());
    let engine = FlowEngine::new();
    let ai = AiRuntime::new(flows.clone(), engine.clone(), runs.clone(), hub.clone());

    let rest_state = RestState {
        flows: flows.clone(),
        agents: agents.clone(),
        runs: runs.clone(),
        hub: hub.clone(),
        engine,
        ai,
    };

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(rest_router(rest_state))
        .with_openapi(FlowAgentApi::openapi())
        .with_metrics(registry, metrics)
        .build();

    Built {
        router,
        flows,
        agents,
        runs,
        hub,
    }
}
