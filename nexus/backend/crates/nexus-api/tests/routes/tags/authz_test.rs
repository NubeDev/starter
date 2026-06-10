//! The tag write/read path enforces the target resource's own authz and rejects
//! unknown ids (WS-13 §3). Tags drive queries via `PageContext.tags`, so the old
//! tenant-only path was an authz hole: a same-tenant caller could tag any — or a
//! nonexistent — dashboard. We prove the fix three ways against a real
//! tenant-bound dashboard:
//!   * `DenyAll` engine ⇒ tagging it is 403 (the grant gate, not RLS — the row
//!     is visible to the tenant).
//!   * `AllowAll` engine ⇒ tagging the same id is 204.
//!   * any engine ⇒ tagging a nonexistent id is 404 (existence checked first, so
//!     a missing id is never silently tagged).

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
use serde_json::json;
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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine,
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

async fn app_with(pool: &sqlx::PgPool, engine: Arc<dyn PolicyEngine>) -> TestApp {
    let router = serve::router(state(pool, engine)).layer(Extension(acme_member()));
    TestApp::spawn(router).await
}

async fn seed_dashboard(pg: &sqlx::PgPool) -> uuid::Uuid {
    dashboard::insert(
        pg,
        "acme",
        &NewDashboard {
            slug: "plant-1".into(),
            name: "Plant 1".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: None,
        },
    )
    .await
    .expect("seed dashboard")
    .id
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tagging_a_dashboard_is_denied_without_edit() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let id = seed_dashboard(&pg).await;

    let app = app_with(&pg, Arc::new(DenyAll)).await;
    let resp = reqwest::Client::new()
        .put(format!("{}/api/v1/tags/dashboard/{id}", app.base_url))
        .json(&json!({ "tags": [{ "key": "building", "value": "b1" }] }))
        .send()
        .await
        .expect("request");
    // The row is visible to the tenant (RLS), so a 403 is the grant gate doing
    // its job — the old tenant-only path would have written the tag (204).
    assert_eq!(resp.status(), 403, "no edit grant ⇒ forbidden");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tagging_a_dashboard_is_allowed_with_edit() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let id = seed_dashboard(&pg).await;

    let app = app_with(&pg, Arc::new(AllowAll)).await;
    let resp = reqwest::Client::new()
        .put(format!("{}/api/v1/tags/dashboard/{id}", app.base_url))
        .json(&json!({ "tags": [{ "key": "building", "value": "b1" }] }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 204, "edit grant ⇒ tags replaced");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tagging_a_nonexistent_dashboard_is_not_found() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    // Even with an all-allowing engine, a dashboard that doesn't exist can't be
    // tagged — existence is checked before authz so a bogus id is a 404, not a
    // silently-written polymorphic tag row.
    let app = app_with(&pg, Arc::new(AllowAll)).await;
    let ghost = uuid::Uuid::nil();
    let resp = reqwest::Client::new()
        .put(format!("{}/api/v1/tags/dashboard/{ghost}", app.base_url))
        .json(&json!({ "tags": [{ "key": "building", "value": "b1" }] }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 404, "nonexistent id ⇒ not found, never tagged");

    drop(app);
}
