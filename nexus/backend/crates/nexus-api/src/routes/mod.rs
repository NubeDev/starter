//! HTTP route wiring. Each submodule owns one resource's routes; this module
//! only composes them. Domain logic stays in the engine and store.

pub mod query;

use axum::Router;

use crate::state::AppState;

/// Compose every product router. `/health`, `/metrics`, and `/openapi.json` are
/// added by `starter_server::ServerBuilder`, not here.
pub fn product_router() -> Router<AppState> {
    Router::new().merge(query::router())
}
