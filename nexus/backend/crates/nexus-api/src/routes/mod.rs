//! HTTP route wiring. Each submodule owns one resource's routes; this module
//! only composes them. Domain logic stays in the engine and store.

pub mod agents;
pub mod ai;
pub mod alerts;
pub mod audit;
pub mod dashboards;
pub mod datasources;
pub mod detections;
pub mod flows;
pub mod folders;
pub mod health;
pub mod ingest;
pub mod insights;
pub mod me;
pub mod nav;
pub mod query;
pub mod query_kinds;
pub mod streams;
pub mod tags;
pub mod undo;
pub mod variables;

use axum::Router;

use crate::state::AppState;

/// Compose every product router. `/health`, `/metrics`, and `/openapi.json` are
/// added by `starter_server::ServerBuilder`, not here; the WS-16 `/livez` and
/// `/readyz` liveness probes are added by `health::router()` below (they read no
/// `Principal`). The binary wraps the returned router in the principal layer so
/// authenticated handlers see the `Principal`.
pub fn product_router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(me::router())
        .merge(query::router())
        .merge(query_kinds::router())
        .merge(streams::router())
        .merge(datasources::router())
        .merge(dashboards::router())
        .merge(flows::router())
        .merge(ingest::router())
        .merge(folders::router())
        .merge(insights::router())
        .merge(nav::router())
        .merge(alerts::router())
        .merge(detections::router())
        .merge(tags::router())
        .merge(agents::router())
        .merge(ai::router())
        .merge(variables::router())
        .merge(audit::router())
        .merge(undo::router())
}
