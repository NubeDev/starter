//! Nav-tree acceptance over the API (WS-13 §4/§6): the tree is access-filtered
//! to nodes the principal holds `view` on, opening a node checks `view` on the
//! node (not the page), and a `dashboard` target is validated to exist in the
//! caller's tenant (a cross-tenant target is rejected). We prove the access
//! filter two ways with the DenyAll/AllowAll engines, and the tenant invariant
//! against a real row.

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
use nexus_store::nav_node::{self, NewNavNode};
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
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

fn group(title: &str) -> NewNavNode {
    NewNavNode {
        parent_id: None,
        title: title.into(),
        sort_order: 0,
        target: json!({ "kind": "group" }),
        context: None,
        icon: None,
        accent: None,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn the_nav_tree_is_access_filtered() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    nav_node::insert(&pg, "acme", &group("Buildings")).await.unwrap();

    // No grant ⇒ the node is filtered out (the row is visible to the tenant via
    // RLS, so an empty list here is the access filter, not a hidden row).
    let denied = app_with(&pg, Arc::new(DenyAll)).await;
    let body: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/nav", denied.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0, "ungranted ⇒ filtered out");
    drop(denied);

    // Granted ⇒ the node appears.
    let allowed = app_with(&pg, Arc::new(AllowAll)).await;
    let body: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/nav", allowed.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1, "granted ⇒ visible");
    drop(allowed);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn opening_a_node_checks_view_on_the_node() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let node = nav_node::insert(&pg, "acme", &group("Building-1")).await.unwrap();

    let denied = app_with(&pg, Arc::new(DenyAll)).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/nav/{}", denied.base_url, node.id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "no node view grant ⇒ forbidden");
    drop(denied);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn creating_a_nav_node_requires_a_kind_wide_grant() {
    // Regression: `create_nav` once persisted unconditionally, so a non-admin
    // (no kind-wide grant) could mint nav nodes. It now runs a collection-level
    // `edit` check — DenyAll ⇒ 403 and nothing is written. The AllowAll/200 side
    // is covered by `a_same_tenant_dashboard_target_is_accepted`.
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;

    let denied = app_with(&pg, Arc::new(DenyAll)).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/nav", denied.base_url))
        .json(&json!({ "title": "ShouldFail", "target": { "kind": "group" } }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "no kind-wide grant ⇒ create forbidden");
    drop(denied);

    // And the row was never persisted (RLS-visible to the tenant, so an empty
    // tenant read proves the insert didn't happen, not that a row is hidden).
    let after = nav_node::list(&pg, "acme").await.unwrap();
    assert!(
        after.iter().all(|n| n.title != "ShouldFail"),
        "denied create must not persist a node"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn a_cross_tenant_dashboard_target_is_rejected() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    // A dashboard in *another* tenant; acme must not be able to mount it.
    let foreign = dashboard::insert(
        &pg,
        "globex",
        &NewDashboard {
            slug: "secret".into(),
            name: "Secret".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: None,
        },
    )
    .await
    .unwrap();

    let app = app_with(&pg, Arc::new(AllowAll)).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/nav", app.base_url))
        .json(&json!({
            "title": "Stolen",
            "target": { "kind": "dashboard", "dashboardId": foreign.id.to_string() },
        }))
        .send()
        .await
        .unwrap();
    // The handler's tenant-scoped existence check (RLS hides globex's row from
    // acme) rejects it as a bad request — never persists a cross-tenant mount.
    assert_eq!(resp.status(), 400, "cross-tenant dashboard target ⇒ rejected");
    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn a_same_tenant_dashboard_target_is_accepted() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let own = dashboard::insert(
        &pg,
        "acme",
        &NewDashboard {
            slug: "energy".into(),
            name: "Energy".into(),
            icon: "Activity".into(),
            accent: "152 76% 44%".into(),
            folder_id: None,
        },
    )
    .await
    .unwrap();

    let app = app_with(&pg, Arc::new(AllowAll)).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/nav", app.base_url))
        .json(&json!({
            "title": "Building-1",
            "target": { "kind": "dashboard", "dashboardId": own.id.to_string() },
            "context": { "values": { "building": "b1" } },
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "own-tenant dashboard target ⇒ created");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["target"]["kind"], "dashboard");
    assert_eq!(body["context"]["values"]["building"], "b1");
    drop(app);
}
