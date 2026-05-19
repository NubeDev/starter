//! Observes request latency, increments the request counter, and
//! maintains the in-flight gauge. Updates the
//! [`StandardMetrics`](starter_observability::metrics::StandardMetrics)
//! handed in at construction time.
//!
//! Exposed as a router-extension helper for the same reason as
//! `request_id` — see that module's comment.

use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;
use starter_observability::metrics::StandardMetrics;

/// Apply the latency-observation middleware to `router`. Each request
/// produces one `requests_total` increment and one
/// `request_duration_seconds` observation; `requests_in_flight` is
/// tracked as a gauge over each handler's lifetime.
pub fn with_latency(router: Router, metrics: Arc<StandardMetrics>) -> Router {
    router.layer(from_fn(move |req: Request<Body>, next: Next| {
        let metrics = metrics.clone();
        async move { observe(metrics, req, next).await }
    }))
}

async fn observe(metrics: Arc<StandardMetrics>, req: Request<Body>, next: Next) -> Response {
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let started = Instant::now();
    metrics.in_flight.inc();
    let resp = next.run(req).await;
    metrics.in_flight.dec();
    let status = resp.status().as_u16().to_string();
    let labels = [method.as_str(), path.as_str(), status.as_str()];
    metrics.requests_total.with_label_values(&labels).inc();
    metrics
        .request_duration_seconds
        .with_label_values(&labels)
        .observe(started.elapsed().as_secs_f64());
    resp
}
