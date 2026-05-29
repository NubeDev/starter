//! `GET /api/v1/admin/cache/specs` — end-to-end.
//!
//! Verifies the per-spec hit/miss snapshot the v0 caching cut
//! exposes for the canary measurement panel:
//!
//! 1. The endpoint returns `{ "specs": [] }` when no cache layer is
//!    wired (developer rigs without extensions).
//! 2. After driving a labelled `get_or_load_labelled` against a
//!    layer threaded into `AdminState`, the endpoint reflects the
//!    counters keyed by spec id.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

use rubix_agent::admin::AdminState;
use rubix_agent::routes::admin::admin_router;

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("body is JSON")
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("build request")
}

#[tokio::test]
async fn cache_specs_empty_when_no_layer_wired() {
    let app = admin_router(AdminState::empty());
    let resp = app
        .oneshot(get("/api/v1/admin/cache/specs"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["specs"], serde_json::json!([]));
}

#[tokio::test]
async fn cache_specs_reflect_labelled_loads() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant);
    let caller = starter_cache::CallerScope::new("tA", "uX");

    // Three calls — one miss + two hits — on a labelled spec.
    for _ in 0..3 {
        let _ = layer
            .get_or_load_labelled::<_, _, std::convert::Infallible>(
                &spec,
                Some("ext.kind.demo"),
                &caller,
                "k",
                || async { Ok(Arc::new(b"x".to_vec())) },
            )
            .await
            .unwrap();
    }

    let app = admin_router(AdminState::empty().with_cache_layer(layer));
    let resp = app
        .oneshot(get("/api/v1/admin/cache/specs"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    let specs = v["specs"].as_array().expect("specs array");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["spec_id"], "ext.kind.demo");
    assert_eq!(specs[0]["hits"], 2);
    assert_eq!(specs[0]["misses"], 1);
    let ratio = specs[0]["hit_ratio"].as_f64().unwrap();
    assert!((ratio - 2.0 / 3.0).abs() < 1e-9, "hit_ratio = {ratio}");
}
