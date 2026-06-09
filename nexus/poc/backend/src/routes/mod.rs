//! HTTP surface: assemble the API router. Wiring only — no handler logic here.

mod catalog;
mod sql;
mod streams;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

/// Build the full `/api` router with permissive CORS for the local UI.
pub fn router() -> Router {
    Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/inputs", get(catalog::inputs::list))
        .route("/api/outputs", get(catalog::outputs::list))
        .route("/api/processors", get(catalog::processors::list))
        .route("/api/buffers", get(catalog::buffers::list))
        .route("/api/plugins", get(catalog::plugins::list))
        .route("/api/streams/validate", post(streams::validate::validate))
        .route("/api/streams/run", post(streams::run::run))
        .route("/api/sql/query", post(sql::query::query))
        .layer(CorsLayer::permissive())
}
