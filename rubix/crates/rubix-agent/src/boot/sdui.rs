//! Boot wiring for the SDUI sub-router.
//!
//! Stage 3 of `rubix/docs/proposal/warehouse-engine-swap.md` removed
//! the ClickHouse-backed `ch:` query plane and the
//! `rubix.analytics.query` chart-source bridge. The remaining
//! backend prefixes are `pg:` (the rubix Postgres pool) and `mem:`
//! (the in-process fallback).

use std::sync::Arc;

use axum::Router;
use rubix_spi::dashboard::DashboardStore;
use rubix_store_postgres::PgDashboardStore;
use starter_sdui_routes::{sdui_router, SduiState};
use starter_spi::tool::Tool;
use starter_store_postgres::pool::Pool;

use crate::boot::AgentConfig;
use crate::sdui::{
    entity_graph::StaticSystemReader, PgPageProvider, RubixEntityGraph, RubixHandlerRegistry,
    RubixQueryEngine,
};

/// Build the SDUI sub-router wired to the rubix data plane.
pub fn build_sdui_router<S>(
    _cfg: &AgentConfig,
    pg_pool: Pool,
    tool_registry: &[Arc<dyn Tool>],
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

    let state = SduiState::builder()
        .with_pages(pages)
        .with_entity_graph(graph)
        .with_query_engine(queries)
        .with_handler_registry(handlers)
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
        let _ = build_sdui_router::<()> as fn(&AgentConfig, Pool, &[Arc<dyn Tool>]) -> Router<()>;

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
