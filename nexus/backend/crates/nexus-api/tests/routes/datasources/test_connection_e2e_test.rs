//! Pre-save connection probe acceptance: POST /datasources/test validates a raw
//! config *before* any datasource is created. Correct credentials report
//! `ok:true` with latency; a wrong secret reports `ok:false` with a sanitized
//! message — and neither path persists a row. This closes the "test only works
//! after save" gap.

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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test"),
            nexus_skills::BrevityMode::Off,
        ),
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
async fn test_connection_probes_raw_config_before_save() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Correct credentials connect — `ok:true`, no row created.
    let ok: Value = client
        .post(format!("{}/api/v1/datasources/test", app.base_url))
        .json(&json!({
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "postgres"
        }))
        .send()
        .await
        .expect("probe")
        .json()
        .await
        .expect("body");
    assert_eq!(ok["ok"], true, "valid credentials probe ok");

    // A wrong secret reports a failed probe — `ok:false` with a message — and
    // still returns 200, because a failed probe is a normal form outcome.
    let bad_resp = client
        .post(format!("{}/api/v1/datasources/test", app.base_url))
        .json(&json!({
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "wrong-password"
        }))
        .send()
        .await
        .expect("probe");
    assert!(bad_resp.status().is_success(), "failed probe is a 200");
    let bad: Value = bad_resp.json().await.expect("body");
    assert_eq!(bad["ok"], false, "wrong secret probe fails");
    assert!(bad["message"].as_str().is_some(), "failure carries a reason");

    // No datasource was persisted by either probe.
    let list: Value = client
        .get(format!("{}/api/v1/datasources", app.base_url))
        .send()
        .await
        .expect("list")
        .json()
        .await
        .expect("body");
    assert!(
        list.as_array().is_none_or(|a| a.is_empty()),
        "probing never creates a datasource"
    );
}
