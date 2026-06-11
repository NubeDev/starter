//! RW-06 query-path acceptance: `POST /api/v1/query` with an attached inline
//! insight runs the result frame through the sandboxed engine before the rows
//! are serialized. We prove three things end to end: a passthrough insight
//! leaves the rows intact, a `filter_gt` insight shrinks the result (and the
//! stats are recomputed to match), and a pathological script (an infinite loop)
//! is stopped by the operation cap and surfaces as a clean 400 — never a hang or
//! a panic.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: std::sync::Arc::new(starter_authz::testing::AllowAll),
        kinds: std::sync::Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: std::sync::Arc::new(nexus_api::kinds::Registry::empty()),

        extensions: nexus_api::extensions::empty_registry(),
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
        canary: Default::default(),
    }
}

async fn seed(pool: &starter_store_postgres::Pool) {
    sqlx::query("CREATE TABLE readings (city text, kw int)")
        .execute(pool.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO readings VALUES ('a', 5), ('b', 15), ('c', 25)")
        .execute(pool.sqlx())
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires docker"]
async fn an_inline_filter_insight_shrinks_the_result() {
    let (pool, _guard) = with_database().await;
    seed(&pool).await;
    let app = TestApp::spawn(serve::router(test_state(pool.sqlx()))).await;

    // Raw query returns three rows; the insight keeps only kw > 10, so two.
    let body: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({
            "sql": "SELECT city, kw FROM readings ORDER BY city",
            "insight": { "script": "df.filter_gt(\"kw\", 10)" }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["rows"].as_array().unwrap().len(), 2, "filter kept kw > 10");
    assert_eq!(
        body["stats"]["row_count"], 2,
        "stats are recomputed from the transformed rows"
    );
    let kws: Vec<i64> = body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["kw"].as_i64().unwrap())
        .collect();
    assert_eq!(kws, [15, 25]);
    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires docker"]
async fn a_passthrough_insight_leaves_rows_intact() {
    let (pool, _guard) = with_database().await;
    seed(&pool).await;
    let app = TestApp::spawn(serve::router(test_state(pool.sqlx()))).await;

    let body: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({
            "sql": "SELECT city, kw FROM readings ORDER BY city",
            "insight": { "script": "df" }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["rows"].as_array().unwrap().len(), 3, "passthrough keeps all rows");
    assert_eq!(body["stats"]["row_count"], 3);
    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires docker"]
async fn a_pathological_script_is_a_clean_400_not_a_hang() {
    let (pool, _guard) = with_database().await;
    seed(&pool).await;
    let app = TestApp::spawn(serve::router(test_state(pool.sqlx()))).await;

    // An infinite loop trips the operation cap; the run layer maps the limit
    // error to a 400, so the request returns rather than hanging the server.
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({
            "sql": "SELECT city, kw FROM readings",
            "insight": { "script": "let n = 0; loop { n += 1; } df" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "a runaway insight is a bad request, not a hang");
    app.shutdown().await;
}
