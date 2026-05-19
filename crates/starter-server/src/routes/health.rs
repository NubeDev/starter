//! `GET /health`. Returns the [`starter_spi::dto::Health`] DTO.

use axum::{routing::get, Json, Router};
use starter_spi::dto::Health;

/// Build the health-check router. Stateless on purpose — the
/// presence of a working axum stack is what's being checked.
pub fn health_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/health", get(handler::<S>))
}

async fn handler<S>() -> Json<Health> {
    Json(Health {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // TODO(ap): wire to a process-start AtomicU64.
    })
}
