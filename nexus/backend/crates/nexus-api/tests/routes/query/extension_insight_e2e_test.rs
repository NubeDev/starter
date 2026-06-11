//! RW-07 query-path acceptance: `POST /api/v1/query` resolving an
//! extension-contributed insight by `insight_name`. This proves the
//! `contributes.insights[]` path end to end at the query stage — a name in the
//! global `nexus_extension_insights` registry resolves (no tenant context),
//! its script runs against the caller's own result rows, and the transform is
//! applied before the rows are serialized. A name that is not registered is a
//! clean 404, never a hang or a panic.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::extension_insight::{self, NewExtensionInsight};
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
async fn an_extension_insight_resolved_by_name_shrinks_the_result() {
    let (pool, _guard) = with_database().await;
    seed(&pool).await;

    // Contribute a global extension insight, exactly as the boot materialise
    // step does for an installed extension's `contributes.insights[]`.
    extension_insight::upsert(
        pool.sqlx(),
        "com.test.ext",
        &NewExtensionInsight {
            name: "com.test.ext.over_ten".into(),
            script: "df.filter_gt(\"kw\", 10)".into(),
            params_schema: None,
        },
    )
    .await
    .unwrap();

    let app = TestApp::spawn(serve::router(test_state(pool.sqlx()))).await;

    // The query resolves the contributed insight by name and applies it to the
    // caller's own rows: three rows in, two out (kw > 10).
    let body: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({
            "sql": "SELECT city, kw FROM readings ORDER BY city",
            "insight": { "insight_name": "com.test.ext.over_ten" }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(
        body["rows"].as_array().unwrap().len(),
        2,
        "the contributed insight kept kw > 10"
    );
    assert_eq!(body["stats"]["row_count"], 2, "stats recomputed from transformed rows");
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
async fn an_unknown_extension_insight_name_is_a_clean_404() {
    let (pool, _guard) = with_database().await;
    seed(&pool).await;
    let app = TestApp::spawn(serve::router(test_state(pool.sqlx()))).await;

    // No insight with this name is registered, so resolution fails as NotFound;
    // the request returns a clean 404 rather than running anything.
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/query", app.base_url))
        .json(&json!({
            "sql": "SELECT city, kw FROM readings",
            "insight": { "insight_name": "com.test.ext.does_not_exist" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "an unknown extension insight name is not found");
    app.shutdown().await;
}
