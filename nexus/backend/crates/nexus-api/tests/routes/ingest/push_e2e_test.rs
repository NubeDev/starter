//! Push-ingest acceptance: a saved `http_ingest` flow accepts pushed JSON over
//! `POST /api/v1/ingest/{flow_id}`; a tiny channel surfaces backpressure as
//! `429 + Retry-After`; and a push to another tenant's flow is a `404`, never a
//! leak. The flow drains to `drop`, so the test needs only the metadata Postgres,
//! not a second datasource container.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::flow::{insert, NewFlow};
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        datasource_pools: Default::default(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        flows: FlowManager::new().expect("flow manager init"),
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-ingest-test"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),

        extensions: nexus_api::extensions::empty_registry(),
        datasource_kinds: Arc::new(nexus_api::datasource_kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        ),
        query_cache: nexus_api::cache::CacheConfig::default().build(),
        quotas: nexus_api::quota::TenantQuotas::new(nexus_api::quota::QuotaConfig::default()),
        rate_limiter: nexus_api::ratelimit::TenantRateLimiter::new(
            nexus_api::ratelimit::RateLimitConfig::default(),
        ),
        canary: Default::default(),
    }
}

fn acme_admin() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Admin,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

/// A push flow body with the given channel capacity.
fn push_flow_body(name: &str, capacity: usize) -> Value {
    json!({
        "name": name,
        "input": { "type": "http_ingest", "capacity": capacity },
        "pipeline": [
            { "type": "json_to_arrow" },
            { "type": "sql", "query": "SELECT v FROM flow" }
        ],
        "output": { "type": "drop" }
    })
}

#[tokio::test]
#[ignore = "requires docker"]
async fn push_lands_rows_and_surfaces_backpressure_and_tenant_isolation() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // A flow owned by a *different* tenant, inserted directly so the cross-tenant
    // probe has a real row to be denied against.
    let other = insert(
        &pool,
        "other",
        &NewFlow {
            name: "foreign".into(),
            input: json!({ "type": "http_ingest" }),
            pipeline: json!([]),
            output: json!({ "type": "drop" }),
            enabled: false,
        },
    )
    .await
    .expect("insert foreign flow");

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Create + start a push flow as acme with a roomy channel.
    let created: Value = client
        .post(format!("{}/api/v1/flows", app.base_url))
        .json(&push_flow_body("pushes", 64))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    let flow_id = created["id"].as_str().expect("id").to_string();
    assert_eq!(
        client
            .post(format!("{}/api/v1/flows/{flow_id}/start", app.base_url))
            .send()
            .await
            .expect("start")
            .status(),
        200
    );
    // Let the source register its channel.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // A push of an array of objects is accepted; the response counts the docs.
    let resp = client
        .post(format!("{}/api/v1/ingest/{flow_id}", app.base_url))
        .json(&json!([{ "v": 1 }, { "v": 2 }, { "v": 3 }]))
        .send()
        .await
        .expect("push");
    assert_eq!(resp.status(), 200, "push accepted");
    let body: Value = resp.json().await.expect("accept body");
    assert_eq!(body["accepted"], 3, "all three documents accepted");

    // The rows reach the (drop) sink — visible through the flow's ingest metrics.
    tokio::time::sleep(Duration::from_millis(120)).await;
    let detail: Value = client
        .get(format!("{}/api/v1/flows/{flow_id}", app.base_url))
        .send()
        .await
        .expect("get flow")
        .json()
        .await
        .expect("detail");
    assert!(
        detail["metrics"]["rows_written"].as_u64().unwrap() >= 3,
        "pushed rows written to the sink"
    );

    // A push to a flow that does not exist (random id) is a 404.
    let missing = client
        .post(format!(
            "{}/api/v1/ingest/{}",
            app.base_url,
            uuid::Uuid::new_v4()
        ))
        .json(&json!({ "v": 9 }))
        .send()
        .await
        .expect("push missing");
    assert_eq!(missing.status(), 404, "unknown flow 404s");

    // A push to another tenant's flow is also a 404 — indistinguishable from a
    // missing flow, so existence never leaks.
    let cross = client
        .post(format!("{}/api/v1/ingest/{}", app.base_url, other.id))
        .json(&json!({ "v": 9 }))
        .send()
        .await
        .expect("push cross-tenant");
    assert_eq!(cross.status(), 404, "cross-tenant push 404s, no leak");

    // Backpressure: a 1-deep flow, then a burst overruns the slot before the sink
    // drains it, so at least one push gets 429 + Retry-After.
    let tiny: Value = client
        .post(format!("{}/api/v1/flows", app.base_url))
        .json(&push_flow_body("tight", 1))
        .send()
        .await
        .expect("create tiny")
        .json()
        .await
        .expect("body");
    let tiny_id = tiny["id"].as_str().expect("id").to_string();
    client
        .post(format!("{}/api/v1/flows/{tiny_id}/start", app.base_url))
        .send()
        .await
        .expect("start tiny");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut saw_429 = false;
    for n in 0..2000 {
        let r = client
            .post(format!("{}/api/v1/ingest/{tiny_id}", app.base_url))
            .json(&json!({ "v": n }))
            .send()
            .await
            .expect("push tight");
        if r.status() == 429 {
            assert!(
                r.headers().contains_key("retry-after"),
                "429 carries Retry-After"
            );
            saw_429 = true;
            break;
        }
    }
    assert!(
        saw_429,
        "a burst over a 1-deep channel eventually backpressures"
    );

    drop(app);
}
