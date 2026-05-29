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
use axum::http::{header, Method, Request, StatusCode};
use serde_json::Value;
use starter_cache::Invalidator;
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

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn post_empty(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(Body::empty())
        .expect("build request")
}

fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::DELETE)
        .uri(uri)
        .body(Body::empty())
        .expect("build request")
}

#[tokio::test]
async fn cache_specs_empty_when_no_layer_wired() {
    let app = admin_router(AdminState::empty());
    let resp = app.oneshot(get("/api/v1/admin/cache/specs")).await.unwrap();
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
    let resp = app.oneshot(get("/api/v1/admin/cache/specs")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let v = body_json(resp).await;
    let specs = v["specs"].as_array().expect("specs array");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["spec_id"], "ext.kind.demo");
    assert_eq!(specs[0]["hits"], 2);
    assert_eq!(specs[0]["misses"], 1);
    let ratio = specs[0]["hit_ratio"].as_f64().unwrap();
    assert!((ratio - 2.0 / 3.0).abs() < 1e-9, "hit_ratio = {ratio}");

    // Latency surface — one miss recorded, hits do not pollute.
    let lat = &specs[0]["load_latency"];
    assert_eq!(lat["count"], 1, "exactly one miss-path load timed");
    let total: u64 = ["le_10ms", "le_100ms", "le_1s", "le_10s", "gt_10s"]
        .iter()
        .map(|b| lat[b].as_u64().unwrap())
        .sum();
    assert_eq!(total, 1, "exactly one bucket incremented");
    assert!(lat["mean_ms"].as_f64().unwrap() >= 0.0);

    // No registry wired in this test → no config visible.
    assert!(specs[0]["config"].is_null());
}

/// When the registry is wired alongside the layer, the response
/// surfaces every registered spec (including those never touched)
/// plus their `config` block so operators can verify the sidecar
/// shape without reading YAML.
#[tokio::test]
async fn cache_specs_join_registry_and_counters() {
    let ext = starter_ext_spi::ExtensionId::new("com.example.ext").expect("ext id");
    // Two specs registered, only one will be touched.
    let touched_spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::User)
        .invalidate_on_table("readings");
    let cold_spec = starter_cache::CacheSpec::ttl(Duration::from_secs(120))
        .scope(starter_cache::CacheScope::Tenant);
    let registry = starter_ext_server::KindCacheRegistry::from_entries([
        (
            (ext.clone(), "com.example.ext.touched".to_string()),
            touched_spec.clone(),
        ),
        (
            (ext.clone(), "com.example.ext.cold".to_string()),
            cold_spec.clone(),
        ),
    ]);
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let caller = starter_cache::CallerScope::new("tA", "uX");

    // Touch only one of the two registered specs.
    for _ in 0..2 {
        let _ = layer
            .get_or_load_labelled::<_, _, std::convert::Infallible>(
                &touched_spec,
                Some("com.example.ext::com.example.ext.touched"),
                &caller,
                "k",
                || async { Ok(Arc::new(b"v".to_vec())) },
            )
            .await
            .unwrap();
    }

    let app = admin_router(
        AdminState::empty()
            .with_cache_layer(layer)
            .with_cache_registry(registry),
    );
    let resp = app.oneshot(get("/api/v1/admin/cache/specs")).await.unwrap();
    let v = body_json(resp).await;
    let specs = v["specs"].as_array().unwrap();

    // BTreeMap order → sorted by spec_id.
    assert_eq!(specs.len(), 2, "both registered specs surfaced");
    let cold = &specs[0];
    assert_eq!(cold["spec_id"], "com.example.ext::com.example.ext.cold");
    assert_eq!(cold["extension"], "com.example.ext");
    assert_eq!(cold["contribute_id"], "com.example.ext.cold");
    assert_eq!(cold["hits"], 0);
    assert_eq!(cold["misses"], 0);
    assert_eq!(cold["config"]["ttl_seconds"], 120);
    assert_eq!(cold["config"]["scope"], "tenant");
    assert_eq!(
        cold["config"]["invalidate_on_tables"],
        serde_json::json!([])
    );

    let touched = &specs[1];
    assert_eq!(
        touched["spec_id"],
        "com.example.ext::com.example.ext.touched"
    );
    assert_eq!(touched["hits"], 1);
    assert_eq!(touched["misses"], 1);
    assert_eq!(touched["config"]["ttl_seconds"], 60);
    assert_eq!(touched["config"]["scope"], "user");
    assert_eq!(
        touched["config"]["invalidate_on_tables"],
        serde_json::json!(["readings"])
    );
}

/// `POST /api/v1/admin/cache/invalidate` returns 503 when the cache
/// layer is not wired — operators must not assume their invalidate
/// took effect.
#[tokio::test]
async fn invalidate_503_when_no_layer_wired() {
    let app = admin_router(AdminState::empty());
    let resp = app
        .oneshot(post_json(
            "/api/v1/admin/cache/invalidate",
            serde_json::json!({ "tags": ["table:foo"] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let v = body_json(resp).await;
    assert_eq!(v["error"], "service_unavailable");
}

/// Happy path: the endpoint fires `invalidate_tags` against the wired
/// layer and reports the number of tags actually fired. Verified by
/// snapshotting the invalidator's tokens before/after.
#[tokio::test]
async fn invalidate_fires_tags_against_wired_layer() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let inv = layer.invalidator();

    // Snapshot the tokens for the tags we're about to fire.
    let tags = vec!["table:alpha".to_string(), "table:beta".to_string()];
    let snap = inv.snapshot_tokens(&tags);
    assert!(inv.tokens_match(&snap), "baseline: tokens match");

    let app = admin_router(AdminState::empty().with_cache_layer(layer));
    let resp = app
        .oneshot(post_json(
            "/api/v1/admin/cache/invalidate",
            serde_json::json!({ "tags": tags }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["invalidated"], 2);

    // After the call, the tokens for both tags must have moved.
    assert!(
        !inv.tokens_match(&snap),
        "tokens must have moved after invalidate"
    );
}

/// Empty `tags` array is accepted and reports `invalidated: 0`. Easier
/// on tooling than a 400 — the only sane no-op shape.
#[tokio::test]
async fn invalidate_empty_tags_is_noop_not_400() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let app = admin_router(AdminState::empty().with_cache_layer(layer));
    let resp = app
        .oneshot(post_json(
            "/api/v1/admin/cache/invalidate",
            serde_json::json!({ "tags": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["invalidated"], 0);
}

/// End-to-end with the dispatcher path: cache a value via the
/// labelled loader, fire invalidate over HTTP, verify next read pays
/// a fresh miss. This is the operational shape — "the data looks
/// stale, curl the invalidate endpoint, verify the next read is
/// fresh".
#[tokio::test]
async fn invalidate_drops_cached_entries_end_to_end() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant)
        .invalidate_on_table("readings");
    let caller = starter_cache::CallerScope::new("tA", "uX");

    // Populate.
    let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    for _ in 0..2 {
        let calls = calls.clone();
        let _ = layer
            .get_or_load_labelled::<_, _, std::convert::Infallible>(
                &spec,
                Some("ext.kind.demo"),
                &caller,
                "k",
                || async move {
                    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(Arc::new(b"v".to_vec()))
                },
            )
            .await
            .unwrap();
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

    // Curl the invalidate endpoint.
    let app = admin_router(AdminState::empty().with_cache_layer(layer.clone()));
    let resp = app
        .oneshot(post_json(
            "/api/v1/admin/cache/invalidate",
            serde_json::json!({ "tags": ["table:readings"] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Next read must pay a fresh miss.
    let calls2 = calls.clone();
    let _ = layer
        .get_or_load_labelled::<_, _, std::convert::Infallible>(
            &spec,
            Some("ext.kind.demo"),
            &caller,
            "k",
            || async move {
                calls2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(Arc::new(b"v2".to_vec()))
            },
        )
        .await
        .unwrap();
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "post-invalidate read must re-run the loader"
    );
}

/// `DELETE /api/v1/admin/cache/tenants/{tenant}` returns 503 when no
/// cache layer is wired (same posture as the invalidate endpoint).
#[tokio::test]
async fn evict_tenant_503_when_no_layer_wired() {
    let app = admin_router(AdminState::empty());
    let resp = app
        .oneshot(delete("/api/v1/admin/cache/tenants/tA"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Happy path: populate two tenants, evict one, verify the other is
/// untouched and the echoed tenant id matches the URL path.
#[tokio::test]
async fn evict_tenant_drops_only_named_tenant() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant);

    for tenant in ["tA", "tB"] {
        let caller = starter_cache::CallerScope::new(tenant, "u");
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", || async {
                Ok(Arc::new(b"v".to_vec()))
            })
            .await
            .unwrap();
    }
    layer.run_pending_tasks().await;
    assert_eq!(layer.tenant_entry_count("tA"), 1);
    assert_eq!(layer.tenant_entry_count("tB"), 1);

    let app = admin_router(AdminState::empty().with_cache_layer(layer.clone()));
    let resp = app
        .oneshot(delete("/api/v1/admin/cache/tenants/tA"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["tenant"], "tA");
    assert_eq!(v["entries_dropped"], 1);

    // B must still have its entry.
    assert_eq!(layer.tenant_entry_count("tA"), 0);
    assert_eq!(layer.tenant_entry_count("tB"), 1);
}

/// `POST /api/v1/admin/cache/invalidate_all` returns 503 when no
/// layer wired — same posture as the other write endpoints.
#[tokio::test]
async fn invalidate_all_503_when_no_layer_wired() {
    let app = admin_router(AdminState::empty());
    let resp = app
        .oneshot(post_empty("/api/v1/admin/cache/invalidate_all"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Happy path: populate two tenants, fire `invalidate_all`, verify
/// every tenant's count drops to zero.
#[tokio::test]
async fn invalidate_all_drops_every_tenant() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant);
    for tenant in ["tA", "tB"] {
        let caller = starter_cache::CallerScope::new(tenant, "u");
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", || async {
                Ok(Arc::new(b"v".to_vec()))
            })
            .await
            .unwrap();
    }
    layer.run_pending_tasks().await;
    let app = admin_router(AdminState::empty().with_cache_layer(layer.clone()));
    let resp = app
        .oneshot(post_empty("/api/v1/admin/cache/invalidate_all"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["entries_dropped"], 2);
    for tenant in ["tA", "tB"] {
        assert_eq!(layer.tenant_entry_count(tenant), 0);
    }
}

/// Unknown tenant returns 200 with `entries_dropped: 0`. A 404 would
/// be precious — the operator's intent ("ensure this tenant has no
/// cache memory") is satisfied either way.
#[tokio::test]
async fn evict_tenant_unknown_returns_zero_not_404() {
    let layer = starter_cache::CacheLayer::new(starter_cache::LayerConfig::default());
    let app = admin_router(AdminState::empty().with_cache_layer(layer));
    let resp = app
        .oneshot(delete("/api/v1/admin/cache/tenants/never-seen"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["tenant"], "never-seen");
    assert_eq!(v["entries_dropped"], 0);
}
