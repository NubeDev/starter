//! `GET /metrics`. Prometheus text exposition format from the shared
//! registry.

use axum::{routing::get, Router};

/// Build the metrics router. Wired up against the consumer's
/// shared prometheus `Registry` once the metrics module is complete.
pub fn metrics_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/metrics", get(handler::<S>))
}

async fn handler<S>() -> &'static str {
    // TODO(ap): encode from prometheus::Registry once
    // starter-observability::metrics::StandardMetrics is wired.
    "# HELP starter_placeholder placeholder until metrics wire\n"
}
