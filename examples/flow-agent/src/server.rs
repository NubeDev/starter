//! Compose the axum router. No auth: every route is open. The example
//! binds to 127.0.0.1 by default and trusts the OS-level boundary
//! (per SCOPE F4).
//!
//! Prefs + i18n are mounted here too. flow-agent has no real auth,
//! so the prefs routes (which require a `Principal`) sit behind
//! `starter_server::auth::with_anonymous_principal` — every request
//! resolves to the same fixed `local-operator` admin principal.
//! Real-product binaries would replace that with `with_principal`
//! + a real `Authenticator`.

use std::sync::Arc;

use axum::Router;
use prometheus::Registry;
use starter_i18n::middleware::accept_language_layer;
use starter_i18n::platform::starter_bundle;
use starter_i18n::routes::router as i18n_router;
use starter_observability::metrics::StandardMetrics;
use starter_prefs::resolver::SystemDefaults;
use starter_prefs::routes::{prefs_router, PrefsRoutesState};
use starter_prefs::store::PgPrefsStore;
use starter_server::auth::{local_operator, with_anonymous_principal};
use starter_server::ServerBuilder;
use starter_store_postgres::flow::PgAgentSessionStore;
use starter_store_postgres::Pool;
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

/// Optional warehouse handle. When the `warehouse` feature is on
/// and the caller supplies a `WarehouseRuntime`, its REST router
/// (per Warehouse SCOPE "REST and SSE surface") is merged into the
/// final assembly.
#[cfg(feature = "warehouse")]
pub type OptWarehouseRuntime = Option<Arc<starter_warehouse::nodes::runtime::WarehouseRuntime>>;

pub fn build(pool: Pool, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Built {
    build_inner(
        pool,
        registry,
        metrics,
        #[cfg(feature = "warehouse")]
        None,
    )
}

#[cfg(feature = "warehouse")]
pub fn build_with_warehouse(
    pool: Pool,
    registry: Arc<Registry>,
    metrics: Arc<StandardMetrics>,
    warehouse: OptWarehouseRuntime,
) -> Built {
    build_inner(pool, registry, metrics, warehouse)
}

fn build_inner(
    pool: Pool,
    registry: Arc<Registry>,
    metrics: Arc<StandardMetrics>,
    #[cfg(feature = "warehouse")] warehouse: OptWarehouseRuntime,
) -> Built {
    let sqlx = pool.sqlx().clone();
    let flows = Arc::new(FlowStore::new(sqlx.clone()));
    let agents = Arc::new(AgentStore::new(sqlx.clone()));
    let runs = Arc::new(RunStore::new(sqlx.clone()));
    let hub = Arc::new(EventHub::new());
    let engine = FlowEngine::new();
    // MEMORY.md Phase M-D — page-builder persistence. Postgres-backed
    // `AgentSessionStore` shares the connection pool with the rest
    // of the example; the schema ships with starter-store-postgres's
    // `AGENT_SESSION_MIGRATION_SOURCE` (wired in `migrations::sources`).
    let agent_sessions: Arc<dyn starter_flow_spi::agent_session::AgentSessionStore> =
        Arc::new(PgAgentSessionStore::new(pool.clone()));

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

    // Prefs surface: Postgres-backed PrefsStore + starter defaults
    // (en-US, UTC, metric). The router is wrapped with
    // `with_anonymous_principal` since flow-agent has no real auth;
    // every request resolves to the same local-operator principal so
    // `/v1/me/preferences` has a stable subject.
    let prefs_store = Arc::new(PgPrefsStore::new(sqlx));
    let prefs_state = PrefsRoutesState::new(prefs_store, SystemDefaults::starter());
    let prefs = with_anonymous_principal::<AppState>(
        prefs_router::<AppState>(prefs_state),
        local_operator("local-operator"),
    );

    // i18n surface: serve the starter-owned catalogs (en + es)
    // layered with the flow-agent-owned chrome (page titles, nav
    // labels) so the React UI can call `useTranslate` for both
    // platform and application strings against the same bundle.
    // The fallback header is enabled in dev so missing translations
    // surface without breaking the no-error guarantee.
    let bundle = {
        let mut b = starter_bundle();
        let en_tag =
            starter_spi::i18n::LanguageTag::parse("en").expect("'en' is a valid BCP-47 tag");
        let es_tag =
            starter_spi::i18n::LanguageTag::parse("es").expect("'es' is a valid BCP-47 tag");
        let en_cat = starter_i18n::catalog::Catalog::from_json_str(include_str!("../i18n/en.json"))
            .expect("embedded flow-agent en.json must be valid");
        let es_cat = starter_i18n::catalog::Catalog::from_json_str(include_str!("../i18n/es.json"))
            .expect("embedded flow-agent es.json must be valid");
        b.extend(en_tag, en_cat);
        b.extend(es_tag, es_cat);
        Arc::new(b)
    };
    let i18n = i18n_router::<AppState>(bundle.clone())
        .layer(accept_language_layer(bundle).with_fallback_header(true));

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(rest_router(rest_state))
        .merge_router(insights_router(insights_state))
        .merge_router(cache_demo_router())
        .merge_router(node_kinds_router::<AppState>(node_kinds_state))
        .merge_router(extensions_router::<AppState>(extensions_mgr))
        .merge_router(prefs)
        .merge_router(i18n)
        .with_openapi(FlowAgentApi::openapi())
        .with_metrics(registry, metrics)
        .build();

    // Warehouse REST surface (W9 / W11 / W14 / W15). When the
    // `warehouse` feature is enabled AND the caller passes a
    // `WarehouseRuntime`, merge its stateless `Router<()>` into
    // the final `axum::Router`. The runtime owns the PG pool +
    // CH client + freshness probe; everything goes through it so
    // W7/W8/W11/W12/W13/W14/W16 are enforced in one place.
    #[cfg(feature = "warehouse")]
    let router = match warehouse {
        Some(rt) => router.merge(starter_warehouse::rest::router(rt)),
        None => router,
    };

    Built {
        router,
        flows,
        agents,
        runs,
        hub,
    }
}
