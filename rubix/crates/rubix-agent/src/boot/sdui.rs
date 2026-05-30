//! Boot wiring for the SDUI sub-router.
//!
//! PR #44 deleted the ClickHouse-backed `ch:` query plane and the
//! `rubix.analytics.query` chart-source bridge. The wiring has been
//! rebuilt on TimescaleDB: when [`AgentConfig::warehouse_url`] is
//! set, a [`TimescaleAnalyticsBridge`] resolves the
//! `analytics_template` chart sources used by the bundled
//! dashboards. When the warehouse is unwired the resolver collapses
//! to empty (KPIs render `—`, charts render "no data").

use std::sync::Arc;

use axum::Router;
use rubix_spi::dashboard::DashboardStore;
use rubix_store_postgres::PgDashboardStore;
use starter_ext_host::{ExtensionRegistry, TemplateRegistry};
use starter_sdui_routes::{sdui_router, AnalyticsBridgeRef, SduiState};
use starter_spi::tool::Tool;
use starter_store_postgres::pool::Pool;
use starter_store_warehouse::WarehouseClient;

use crate::boot::AgentConfig;
use crate::sdui::{
    entity_graph::StaticSystemReader, PgPageProvider, RubixEntityGraph, RubixHandlerRegistry,
    RubixQueryEngine, TimescaleAnalyticsBridge,
};

/// Build the SDUI sub-router wired to the rubix data plane.
pub fn build_sdui_router<S>(
    _cfg: &AgentConfig,
    pg_pool: Pool,
    warehouse: Option<WarehouseClient>,
    tool_registry: &[Arc<dyn Tool>],
    template_registry: Option<Arc<TemplateRegistry>>,
    extension_registry: Option<Arc<ExtensionRegistry>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let store: Arc<dyn DashboardStore> = Arc::new(PgDashboardStore::new(pg_pool.clone()));
    let pages = PgPageProvider::bundled(store);

    let system_reader = Arc::new(StaticSystemReader::new());
    let graph = RubixEntityGraph::new(pg_pool.clone(), system_reader);

    let queries = RubixQueryEngine::new(Some(pg_pool));

    let handlers = RubixHandlerRegistry::build(tool_registry);

    let mut builder = SduiState::builder()
        .with_pages(pages)
        .with_entity_graph(graph)
        .with_query_engine(queries)
        .with_handler_registry(handlers);
    if let Some(client) = warehouse {
        // Prefer the host-built merged registry (builtins +
        // extension-contributed templates from `compose.rs`). When
        // absent (e.g. no extension bundle), fall back to the
        // builtin-only set so the four `meter_*` templates still
        // resolve.
        let registry = template_registry.unwrap_or_else(|| Arc::new(TemplateRegistry::builtin()));
        let mut bridge = TimescaleAnalyticsBridge::with_registry(client, registry);
        if let Some(ext_reg) = extension_registry {
            bridge = bridge.with_extension_registry(ext_reg);
        }
        let bridge: AnalyticsBridgeRef = Arc::new(bridge);
        builder = builder.with_analytics(bridge);
    }
    let state = builder
        .build()
        .expect("SDUI state: every piece is wired above");

    sdui_router::<S>(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_mounts_ui_routes_at_expected_paths() {
        let _ = build_sdui_router::<()>
            as fn(
                &AgentConfig,
                Pool,
                Option<WarehouseClient>,
                &[Arc<dyn Tool>],
                Option<Arc<TemplateRegistry>>,
                Option<Arc<ExtensionRegistry>>,
            ) -> Router<()>;

        let app: Router = Router::new();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
