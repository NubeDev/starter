//! PR 0 — `static_assets::mount` smoke test.
//!
//! Serves a temp dir with an `index.html` and an asset file, asserts:
//!   (a) the asset is served with the correct `Content-Type`,
//!   (b) an unknown sub-path falls back to `index.html` with
//!       `text/html`,
//!   (c) starter-owned routes (`/health`) still work when an
//!       opt-in static mount is added alongside them.

#![cfg(feature = "testing")]

use std::fs;
use std::sync::Arc;

use prometheus::Registry;
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};

#[derive(Clone)]
struct EmptyState;

fn write_dist() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::write(
        tmp.path().join("index.html"),
        "<!doctype html><title>spa</title>",
    )
    .unwrap();
    fs::write(tmp.path().join("app.js"), "console.log('hi');").unwrap();
    tmp
}

#[tokio::test]
async fn static_assets_serves_files_and_falls_back_to_index() {
    let tmp = write_dist();
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));

    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry, metrics)
        .with_static_assets("/ui", tmp.path().to_path_buf())
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // (a) asset served with correct Content-Type
    let resp = client
        .get(format!("{}/ui/app.js", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        ct.starts_with("application/javascript") || ct.starts_with("text/javascript"),
        "unexpected content-type for app.js: {ct}",
    );
    assert_eq!(resp.text().await.unwrap(), "console.log('hi');");

    // (b) unknown path falls back to index.html with text/html
    let resp = client
        .get(format!("{}/ui/does/not/exist", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(ct.starts_with("text/html"), "expected html, got {ct}");
    let body = resp.text().await.unwrap();
    assert!(body.contains("<title>spa</title>"));

    // (c) existing starter routes still work
    let resp = client
        .get(format!("{}/health", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    app.shutdown().await;
}
