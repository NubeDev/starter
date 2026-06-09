//! `GET`/`PATCH /api/v1/me/preferences` against a real, tenant-bound store.
//!
//! The prefs store runs under the non-BYPASSRLS `nexus_runtime` role and is NOT
//! RLS-guarded (it runs outside `tenant_tx`); isolation is route-pinned to
//! `principal.tenant_id`. These tests prove the contract end-to-end: an
//! unpatched caller resolves the system defaults, a PATCH persists and resolves
//! back, and a second tenant with the same user id cannot see the first's row.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;

fn state(pool: &sqlx::PgPool) -> AppState {
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

fn member(subject: &str, tenant: &str) -> Principal {
    Principal {
        subject: subject.into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some(tenant.into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

async fn app_for(pool: &sqlx::PgPool, principal: Principal) -> TestApp {
    let router = serve::router(state(pool)).layer(Extension(principal));
    TestApp::spawn(router).await
}

#[tokio::test]
#[ignore = "requires docker"]
async fn get_unpatched_resolves_system_defaults() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    let app = app_for(&pg, member("alice", "acme")).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/me/preferences", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    // Starter system defaults: metric / Celsius / UTC.
    assert_eq!(body["unit_system"], "metric");
    assert_eq!(body["temperature_unit"], "celsius");
    assert_eq!(body["timezone"], "UTC");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn patch_persists_and_resolves_back() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    let app = app_for(&pg, member("alice", "acme")).await;
    let client = reqwest::Client::new();
    let patched: serde_json::Value = client
        .patch(format!("{}/api/v1/me/preferences", app.base_url))
        .json(&serde_json::json!({ "temperature_unit": "fahrenheit" }))
        .send()
        .await
        .expect("patch")
        .json()
        .await
        .expect("json");
    assert_eq!(patched["temperature_unit"], "fahrenheit");

    // A fresh GET reflects the persisted row.
    let got: serde_json::Value = client
        .get(format!("{}/api/v1/me/preferences", app.base_url))
        .send()
        .await
        .expect("get")
        .json()
        .await
        .expect("json");
    assert_eq!(got["temperature_unit"], "fahrenheit");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn same_user_id_in_another_tenant_is_isolated() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let client = reqwest::Client::new();

    // alice@acme sets Fahrenheit.
    let acme = app_for(&pg, member("alice", "acme")).await;
    client
        .patch(format!("{}/api/v1/me/preferences", acme.base_url))
        .json(&serde_json::json!({ "temperature_unit": "fahrenheit" }))
        .send()
        .await
        .expect("patch acme");
    drop(acme);

    // alice@globex — same subject, different tenant — must still see defaults.
    let globex = app_for(&pg, member("alice", "globex")).await;
    let got: serde_json::Value = client
        .get(format!("{}/api/v1/me/preferences", globex.base_url))
        .send()
        .await
        .expect("get globex")
        .json()
        .await
        .expect("json");
    assert_eq!(
        got["temperature_unit"], "celsius",
        "tenant pinning must isolate the same user id across tenants"
    );

    drop(globex);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn no_tenant_binding_is_unauthorized() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    let mut p = member("alice", "acme");
    p.tenant_id = None;
    let app = app_for(&pg, p).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/me/preferences", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 401);

    drop(app);
}
