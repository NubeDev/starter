//! Stored-insight acceptance over the API (RW-06 §2): CRUD round-trips through
//! the assembled router, a script that does not compile is rejected at save
//! time (never persisted un-runnable), and the collection is RLS-isolated — a
//! caller in one tenant never sees, fetches, or deletes another tenant's
//! insight. The access engine is `AllowAll` so a 404 here is the tenant
//! boundary, not a missing grant.

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
use serde_json::{json, Value};
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
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),
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

fn member(tenant: &str) -> Principal {
    Principal {
        subject: format!("{tenant}-user"),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some(tenant.into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

async fn app_for(pool: &sqlx::PgPool, tenant: &str) -> TestApp {
    let router = serve::router(state(pool)).layer(Extension(member(tenant)));
    TestApp::spawn(router).await
}

/// A trivial valid insight script: the identity transform passes the frame
/// through unchanged, so it compiles and runs.
const PASSTHROUGH: &str = "df";

#[tokio::test]
#[ignore = "requires docker"]
async fn create_then_get_round_trips() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let app = app_for(&pg, "acme").await;

    let created: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/insights", app.base_url))
        .json(&json!({ "name": "smooth", "script": PASSTHROUGH }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(created["name"], "smooth");
    let id = created["id"].as_str().unwrap();

    let fetched: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/insights/{}", app.base_url, id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["id"], created["id"]);
    assert_eq!(fetched["script"], PASSTHROUGH);
    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires docker"]
async fn a_script_that_does_not_compile_is_rejected() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let app = app_for(&pg, "acme").await;

    // A syntax error must never reach the store: the save is a 400, and the
    // collection stays empty.
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/insights", app.base_url))
        .json(&json!({ "name": "broken", "script": "let x = ;" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "an uncompilable script is rejected");

    let list: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/insights", app.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0, "nothing was persisted");
    app.shutdown().await;
}

#[tokio::test]
#[ignore = "requires docker"]
async fn insights_are_rls_isolated_across_tenants() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    // acme creates an insight.
    let acme = app_for(&pg, "acme").await;
    let created: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/insights", acme.base_url))
        .json(&json!({ "name": "acme-only", "script": PASSTHROUGH }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    acme.shutdown().await;

    // globex must not see it in the list, fetch it, or delete it — every path
    // is RLS-scoped, so the foreign row is invisible (404), not forbidden.
    let globex = app_for(&pg, "globex").await;
    let list: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/insights", globex.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0, "no cross-tenant row in list");

    let get = reqwest::Client::new()
        .get(format!("{}/api/v1/insights/{}", globex.base_url, id))
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 404, "cross-tenant fetch is a 404");

    let del = reqwest::Client::new()
        .delete(format!("{}/api/v1/insights/{}", globex.base_url, id))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), 404, "cross-tenant delete is a 404");
    globex.shutdown().await;

    // The row still exists for acme.
    let acme = app_for(&pg, "acme").await;
    let get = reqwest::Client::new()
        .get(format!("{}/api/v1/insights/{}", acme.base_url, id))
        .send()
        .await
        .unwrap();
    assert_eq!(get.status(), 200, "the owner can still fetch it");
    acme.shutdown().await;
}
