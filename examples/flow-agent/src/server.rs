//! Compose the axum router. No auth: every route is open. The example
//! binds to 127.0.0.1 by default and trusts the OS-level boundary
//! (per SCOPE F4).

use std::sync::Arc;

use axum::Router;
use prometheus::Registry;
use starter_observability::metrics::StandardMetrics;
use starter_server::ServerBuilder;
use starter_store_sqlite::flow::SqliteAgentSessionStore;
use starter_store_sqlite::Pool;
use utoipa::OpenApi;

use crate::ai_runtime::AiRuntime;
use crate::cache_demo::router as cache_demo_router;
use crate::extensions::{router as extensions_router, ExtensionManager};
use crate::flow_engine::FlowEngine;
use crate::insights_mock::{
    default_fixtures_dir, router as insights_router, InsightsFixtures, InsightsState,
};
use crate::node_kinds::{router as node_kinds_router, NodeKindsState};
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
    // MEMORY.md Phase M-D — page-builder persistence. SQLite-backed
    // `AgentSessionStore` shares the connection pool with the rest
    // of the example; the schema ships with starter-flow's
    // `FLOW_MIGRATION_SOURCE` (wired in `migrations::sources`).
    let agent_sessions: Arc<dyn starter_flow_spi::agent_session::AgentSessionStore> =
        Arc::new(SqliteAgentSessionStore::new(pool.clone()));

    // Insights mock-up surface (INSIGHTS-MOCKUP.md). Fixture files
    // are loaded on startup; missing fixtures degrade to an empty
    // in-memory store so the rest of the server keeps booting.
    let insights_dir = default_fixtures_dir();
    let insights_data = InsightsFixtures::load(&insights_dir).unwrap_or_else(|err| {
        tracing::warn!(
            target: "flow_agent::insights_mock",
            dir = %insights_dir.display(),
            error = %err,
            "insights fixtures not loaded; mock surface will return empty arrays",
        );
        InsightsFixtures {
            root: insights_dir.clone(),
            ..InsightsFixtures::default()
        }
    });
    let insights_state = InsightsState::new(insights_data);
    let ai = AiRuntime::new(flows.clone(), engine.clone(), runs.clone(), hub.clone())
        .with_insights(insights_state.clone());

    let rest_state = RestState {
        flows: flows.clone(),
        agents: agents.clone(),
        runs: runs.clone(),
        hub: hub.clone(),
        engine,
        ai,
        agent_sessions,
    };

    // Slice A of `DOCS/extensions/scope/FLOW-NODES.md`: ship the
    // descriptor surface. Slice B's `POST /admin/extensions/reload`
    // will swap the dynamic half through the `ArcSwap` inside this
    // state.
    let node_kinds_state = NodeKindsState::with_builtins();

    // Slice B (`DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-6):
    // pull the extensions root from the `STARTER_EXTENSIONS_DIR`
    // env var (the demo's default location is
    // `examples/flow-agent/extensions/`). The manager scans on
    // construction and exposes `POST /admin/extensions/reload` so an
    // operator can hot-load new bundles after edit. If the env var
    // is unset, fall back to the demo dir alongside the binary.
    let extensions_root = std::env::var("STARTER_EXTENSIONS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extensions")
        });
    let extensions_mgr =
        ExtensionManager::bootstrap(extensions_root, node_kinds_state.clone(), hub.clone());

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(rest_router(rest_state))
        .merge_router(insights_router(insights_state))
        .merge_router(cache_demo_router())
        .merge_router(node_kinds_router::<AppState>(node_kinds_state))
        .merge_router(extensions_router::<AppState>(extensions_mgr))
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
