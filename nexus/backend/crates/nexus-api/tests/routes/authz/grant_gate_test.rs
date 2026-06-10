//! Persisted-resource handlers are gated by a grant check on the immutable id,
//! with RLS underneath. We prove the gate two ways against a real, tenant-bound
//! dashboard: a `DenyAll` engine turns an otherwise-valid `GET` into a 403,
//! while an `AllowAll` engine lets the same request through to 200. The
//! distinction matters — both reach the same row (RLS lets them), so a 403 here
//! is the grant layer doing its job, not RLS hiding the row.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::dashboard::{self, NewDashboard};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use starter_authz::testing::{AllowAll, DenyAll};
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::PolicyEngine;
use starter_store_postgres::testing::with_database;

fn state(pool: &sqlx::PgPool, engine: Arc<dyn PolicyEngine>) -> AppState {
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
        engine,
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

fn acme_member() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec!["ops".into()],
        tenant_scope: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

/// The product router with a fixed principal layered on, bypassing the real
/// authenticator so the test exercises the grant gate in isolation.
async fn app_with(pool: &sqlx::PgPool, engine: Arc<dyn PolicyEngine>) -> TestApp {
    let router = serve::router(state(pool, engine)).layer(Extension(acme_member()));
    TestApp::spawn(router).await
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dashboard_get_is_denied_without_a_grant() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    dashboard::insert(
        &pg,
        "acme",
        &NewDashboard {
            slug: "plant-1".into(),
            name: "Plant 1".into(),
        },
    )
    .await
    .expect("seed dashboard");

    let app = app_with(&pg, Arc::new(DenyAll)).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/dashboards/plant-1", app.base_url))
        .send()
        .await
        .expect("request");
    // The row is visible to the tenant (RLS), so a 403 here is the grant gate,
    // not a hidden row (which would be 404).
    assert_eq!(resp.status(), 403, "no grant ⇒ forbidden, not served");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dashboard_get_is_served_with_a_grant() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    dashboard::insert(
        &pg,
        "acme",
        &NewDashboard {
            slug: "plant-1".into(),
            name: "Plant 1".into(),
        },
    )
    .await
    .expect("seed dashboard");

    let app = app_with(&pg, Arc::new(AllowAll)).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/dashboards/plant-1", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200, "granted ⇒ served");

    drop(app);
}
