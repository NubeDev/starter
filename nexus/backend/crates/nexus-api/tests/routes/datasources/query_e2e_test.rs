//! Datasource query acceptance: a Postgres datasource registered over the API is
//! queried end-to-end through `POST /datasources/:id/query` — the M4 "postgres as
//! a real datasource" path. The datasource points back at the test container with
//! its credentials sealed, so the request exercises register → seal → grant-gate →
//! decrypt → connect (cached) → guarded query, exactly as a user's would.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
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
        sessions: nexus_api::agents::SessionRunner::new(std::env::temp_dir().join("nexus-knowledge-test"), nexus_skills::BrevityMode::Off),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
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

#[tokio::test]
#[ignore = "requires docker"]
async fn registered_postgres_datasource_is_queryable_end_to_end() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    // The data the datasource will serve, seeded on the target DB.
    sqlx::query("CREATE TABLE readings (id int primary key, watt double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO readings VALUES (1, 240.5), (2, 12.0)")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Register the datasource over the API — pointing back at the container, with
    // the container's own credentials as the (sealed) secret.
    let created: Value = client
        .post(format!("{}/api/v1/datasources", app.base_url))
        .json(&json!({
            "name": "self",
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "postgres"
        }))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    let id = created["id"].as_str().expect("id");

    // Query it through the guarded per-datasource route.
    let result: Value = client
        .post(format!("{}/api/v1/datasources/{id}/query", app.base_url))
        .json(&json!({ "sql": "SELECT id, watt FROM readings ORDER BY id" }))
        .send()
        .await
        .expect("query")
        .json()
        .await
        .expect("body");

    assert_eq!(result["stats"]["row_count"], 2);
    assert_eq!(result["rows"][1]["watt"], 12.0);

    // A second query reuses the cached pool (no second connect) and still works.
    let again: Value = client
        .post(format!("{}/api/v1/datasources/{id}/query", app.base_url))
        .json(&json!({ "sql": "SELECT count(*) AS n FROM readings" }))
        .send()
        .await
        .expect("query")
        .json()
        .await
        .expect("body");
    assert_eq!(again["rows"][0]["n"], 2);

    // The read-only guard rejects a write through the datasource route — a 400,
    // not a silent success.
    let write = client
        .post(format!("{}/api/v1/datasources/{id}/query", app.base_url))
        .json(&json!({ "sql": "INSERT INTO readings VALUES (3, 0)" }))
        .send()
        .await
        .expect("write attempt");
    assert_eq!(write.status(), 400);

    drop(app);
}
