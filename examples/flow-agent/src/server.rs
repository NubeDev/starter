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
use starter_prefs::store::SqlitePrefsStore;
use starter_server::auth::{local_operator, with_anonymous_principal};
use starter_server::ServerBuilder;
use starter_store_sqlite::flow::SqliteAgentSessionStore;
use starter_store_sqlite::Pool;
use utoipa::OpenApi;

use crate::ai_runtime::AiRuntime;
use crate::cache_demo::router as cache_demo_router;
use crate::flow_engine::FlowEngine;
use crate::insights_mock::{
    default_fixtures_dir, router as insights_router, InsightsFixtures, InsightsState,
};
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

    // Prefs surface: SQLite-backed PrefsStore + starter defaults
    // (en-US, UTC, metric). The router is wrapped with
    // `with_anonymous_principal` since flow-agent has no real auth;
    // every request resolves to the same local-operator principal so
    // `/v1/me/preferences` has a stable subject.
    let prefs_store = Arc::new(SqlitePrefsStore::new(pool.sqlx().clone()));
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
        let en_tag = starter_spi::i18n::LanguageTag::parse("en")
            .expect("'en' is a valid BCP-47 tag");
        let es_tag = starter_spi::i18n::LanguageTag::parse("es")
            .expect("'es' is a valid BCP-47 tag");
        let en_cat = starter_i18n::catalog::Catalog::from_json_str(include_str!(
            "../i18n/en.json"
        ))
        .expect("embedded flow-agent en.json must be valid");
        let es_cat = starter_i18n::catalog::Catalog::from_json_str(include_str!(
            "../i18n/es.json"
        ))
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
        .merge_router(prefs)
        .merge_router(i18n)
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
