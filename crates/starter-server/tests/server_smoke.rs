//! End-to-end smoke: build a ServerBuilder against an empty state, hit
//! `/health` and `/metrics`, verify request-id echo and metric labels.

#![cfg(feature = "testing")]

use std::sync::Arc;

use prometheus::Registry;
use starter_observability::{metrics::StandardMetrics, middleware::REQUEST_ID_HEADER};
use starter_server::{testing::TestApp, ServerBuilder};

#[derive(Clone)]
struct EmptyState;

#[tokio::test]
async fn health_returns_ok_and_echoes_request_id() {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));

    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry.clone(), metrics.clone())
        .build();
    let app = TestApp::spawn(router).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/health", app.base_url))
        .header(REQUEST_ID_HEADER, "11111111-2222-3333-4444-555555555555")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let echoed = resp
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    assert_eq!(
        echoed.as_deref(),
        Some("11111111-2222-3333-4444-555555555555"),
    );

    app.shutdown().await;
}

#[tokio::test]
async fn metrics_exposes_prometheus_body_after_traffic() {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));

    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry.clone(), metrics.clone())
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let _ = client
        .get(format!("{}/health", app.base_url))
        .send()
        .await
        .unwrap();

    let body = client
        .get(format!("{}/metrics", app.base_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.contains("starter_requests_total"));
    assert!(body.contains("starter_request_duration_seconds"));
    assert!(body.contains("path=\"/health\""));

    app.shutdown().await;
}

#[tokio::test]
async fn openapi_route_serves_doc_when_provided() {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));

    let doc = utoipa::openapi::OpenApiBuilder::new()
        .info(utoipa::openapi::InfoBuilder::new().title("smoke").build())
        .build();

    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry, metrics)
        .with_openapi(doc)
        .build();
    let app = TestApp::spawn(router).await;

    let resp: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/openapi.json", app.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["info"]["title"], "smoke");

    app.shutdown().await;
}
