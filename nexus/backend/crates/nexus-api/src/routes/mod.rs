//! HTTP route wiring. Each submodule owns one resource's routes; this module
//! only composes them. Domain logic stays in the engine and store.

pub mod agents;
pub mod alerts;
pub mod dashboards;
pub mod datasources;
pub mod flows;
pub mod me;
pub mod query;
pub mod streams;
pub mod tags;
pub mod variables;

use axum::Router;

use crate::state::AppState;

/// Compose every product router. `/health`, `/metrics`, and `/openapi.json` are
/// added by `starter_server::ServerBuilder`, not here. The binary wraps the
/// returned router in the principal layer so handlers see the `Principal`.
pub fn product_router() -> Router<AppState> {
    Router::new()
        .merge(me::router())
        .merge(query::router())
        .merge(streams::router())
        .merge(datasources::router())
        .merge(dashboards::router())
        .merge(flows::router())
        .merge(alerts::router())
        .merge(tags::router())
        .merge(agents::router())
        .merge(variables::router())
}
