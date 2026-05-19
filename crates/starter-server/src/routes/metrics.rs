//! `GET /metrics`. Prometheus text exposition format from the shared
//! registry.
//!
//! The handler closes over the registry rather than threading it
//! through axum's state — that way the metrics route can be merged
//! into a consumer router with any `AppState` without state coercions.

use std::sync::Arc;

use axum::{http::header, response::IntoResponse, routing::get, Router};
use prometheus::{Encoder, Registry, TextEncoder};

/// Build the metrics router. The handler encodes the registry on each
/// request — Prometheus' default scrape frequency is once every 15 s
/// so the per-request encode cost is negligible.
pub fn metrics_router<S: Clone + Send + Sync + 'static>(registry: Arc<Registry>) -> Router<S> {
    Router::new().route("/metrics", get(move || serve(registry.clone())))
}

async fn serve(registry: Arc<Registry>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    if let Err(e) = encoder.encode(&registry.gather(), &mut buf) {
        return (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("metrics encode failed: {e}"),
        )
            .into_response();
    }
    (
        [(header::CONTENT_TYPE, encoder.format_type())],
        String::from_utf8(buf).unwrap_or_default(),
    )
        .into_response()
}
