//! M0 acceptance over HTTP: `POST /api/v1/query` returns real rows from a real
//! Postgres through the assembled router. This is the end-to-end seam — request
//! body in, JSON rows out — that M0 exists to prove.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use serde_json::json;
use starter_server::testing::TestApp;
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
        flows: nexus_engine::FlowManager::new().expect("flow manager init"),
        sessions: nexus_api::agents::SessionRunner::new(std::env::temp_dir().join("nexus-knowledge-test"), nexus_skills::BrevityMode::Off),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: std::sync::Arc::new(starter_authz::testing::AllowAll),
        kinds: std::sync::Arc::new(nexus_api::kinds::Registry::empty()),
        datasource_kinds: std::sync::Arc::new(nexus_api::datasource_kinds::Registry::empty()),
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
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn post_query_returns_real_rows() {
    let (pool, _guard) = with_database().await;
    sqlx::query("CREATE TABLE demo_bi (city text, sales int)")
        .execute(pool.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO demo_bi VALUES ('berlin', 10), ('madrid', 20)")
        .execute(pool.sqlx())
        .await
        .unwrap();

    let state = test_state(pool.sqlx());
    let app = TestApp::spawn(serve::router(state)).await;

    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({ "sql": "SELECT city, sales FROM demo_bi ORDER BY city" }))
        .send()
        .await
        .expect("request sent");
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(body["stats"]["row_count"], 2);
    assert_eq!(body["rows"][0]["city"], "berlin");
    assert_eq!(body["rows"][1]["sales"], 20);
    let col_names: Vec<&str> = body["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(col_names, ["city", "sales"]);

    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires docker"]
async fn post_query_rejects_a_write_with_400() {
    let (pool, _guard) = with_database().await;
    sqlx::query("CREATE TABLE t (n int)")
        .execute(pool.sqlx())
        .await
        .unwrap();

    let state = test_state(pool.sqlx());
    let app = TestApp::spawn(serve::router(state)).await;

    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({ "sql": "INSERT INTO t VALUES (1)" }))
        .send()
        .await
        .expect("request sent");
    assert_eq!(resp.status(), 400, "a write is rejected as a bad request");

    app.shutdown().await;
}
