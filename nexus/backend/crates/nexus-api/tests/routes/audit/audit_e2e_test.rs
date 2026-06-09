//! Audit/undo substrate end-to-end (WS-12) against a real, RLS-bound store.
//!
//! Proves the contract the way production runs (non-BYPASSRLS `nexus_runtime`
//! role, single reused connection):
//! - a recorded change surfaces on `GET /api/v1/audit` with a non-null `before`
//!   on an update (the silently-empty-audit-row failure mode);
//! - a second tenant cannot see the first's rows (RLS tenant isolation);
//! - audit read is admin-gated (a writer gets 403);
//! - undo of a recorded dashboard update restores the `before` snapshot.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::changelog::{actor_from, record};
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::reversible::dashboard_snapshot_json;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::dashboard::{self, DashboardPatch, NewDashboard};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use nexus_api::authz::KIND_DASHBOARD;
use starter_spi::authz::ResourceRef;

const ENVELOPE_KEY: &[u8] = b"0123456789abcdef0123456789abcdef";

fn state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        datasource_pools: Default::default(),
        envelope: Envelope::new(ENVELOPE_KEY, 1).unwrap(),
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
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(ENVELOPE_KEY, 1).unwrap(),
        ),
    }
}

fn admin(subject: &str, tenant: &str) -> Principal {
    principal(subject, tenant, Role::Admin)
}

fn writer(subject: &str, tenant: &str) -> Principal {
    principal(subject, tenant, Role::Writer)
}

fn principal(subject: &str, tenant: &str, role: Role) -> Principal {
    Principal {
        subject: subject.into(),
        role,
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

fn new_dashboard(slug: &str, name: &str) -> NewDashboard {
    NewDashboard {
        slug: slug.into(),
        name: name.into(),
        icon: "gauge".into(),
        accent: "152 76% 44%".into(),
    }
}

/// Create a dashboard, rename it, and record the update through the C6 helper —
/// the path a real mutating handler follows. Returns the dashboard id and the
/// recorded `before`/`after` so the test can assert the audit row matches.
async fn record_a_rename(state: &AppState, tenant: &str) -> Uuid {
    record_a_rename_slug(state, tenant, "ops").await
}

/// As [`record_a_rename`] but with a caller-chosen slug, so a test recording two
/// dashboards in one tenant does not collide on the unique slug.
async fn record_a_rename_slug(state: &AppState, tenant: &str, slug: &str) -> Uuid {
    let created = dashboard::insert(&state.metadata, tenant, &new_dashboard(slug, "Ops"))
        .await
        .expect("create dashboard");
    let before = dashboard_snapshot_json(&created);

    let renamed = dashboard::update(
        &state.metadata,
        tenant,
        created.id,
        &DashboardPatch {
            name: Some("Operations".into()),
            ..Default::default()
        },
    )
    .await
    .expect("update dashboard")
    .expect("dashboard visible");
    let after = dashboard_snapshot_json(&renamed);

    let draft = ChangeDraft::update(
        ResourceRef::row(KIND_DASHBOARD, created.id.to_string()).with_tenant(tenant),
        before,
        after,
    );
    record(
        &state.changelog.registry,
        state.metadata.clone(),
        tenant,
        actor_from(&admin("alice", tenant)),
        draft,
    )
    .await
    .expect("record")
    .expect("dashboard is a registered reversible kind");

    created.id
}

#[tokio::test]
#[ignore = "requires docker"]
async fn recorded_change_surfaces_in_audit_with_before_snapshot() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let id = record_a_rename(&st, "acme").await;

    let app = app_for(&pg, admin("alice", "acme")).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/api/v1/audit", app.base_url))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .expect("json");

    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "exactly the one recorded change");
    let row = &items[0];
    assert_eq!(row["resource"]["kind"], KIND_DASHBOARD);
    assert_eq!(row["resource"]["id"], id.to_string());
    assert_eq!(row["op"], "update");
    // The silently-empty-audit-row guard: an update MUST carry its pre-state.
    assert_eq!(row["before"]["name"], "Ops");
    assert_eq!(row["after"]["name"], "Operations");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn audit_is_tenant_isolated() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    record_a_rename(&st, "acme").await;

    // A different tenant's admin sees none of acme's rows (RLS).
    let app = app_for(&pg, admin("bob", "other")).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/api/v1/audit", app.base_url))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    assert_eq!(
        body["items"].as_array().expect("items").len(),
        0,
        "another tenant's admin must not see acme's audit rows",
    );

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn audit_read_is_admin_gated() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;

    let app = app_for(&pg, writer("carol", "acme")).await;
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/audit", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 403, "a non-admin cannot read the audit log");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn retention_sweep_prunes_aged_rows_but_keeps_recent() {
    use nexus_api::changelog::{prune, RetentionPolicy};

    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    // Two recorded changes: one we backdate past the horizon, one left recent.
    record_a_rename_slug(&st, "acme", "recent").await;
    let aged_id = record_a_rename_slug(&st, "acme", "aged").await;

    // Backdate the aged dashboard's audit rows beyond a 30-day horizon. The owner
    // pool bypasses RLS, standing in for "rows that have simply gotten old".
    sqlx::query("UPDATE nexus_changes SET at = now() - interval '400 days' WHERE resource_id = $1")
        .bind(aged_id.to_string())
        .execute(admin_pool.sqlx())
        .await
        .expect("backdate aged rows");

    std::env::set_var("NEXUS_AUDIT_RETENTION_DAYS", "30");
    let policy = RetentionPolicy::from_env();
    std::env::remove_var("NEXUS_AUDIT_RETENTION_DAYS");

    let pruned = prune::run_once(&st, policy).await.expect("prune sweep");
    assert_eq!(pruned, 1, "exactly the one aged row is pruned");

    // The recent change still surfaces; the aged one is gone.
    let app = app_for(&pg, admin("alice", "acme")).await;
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/api/v1/audit", app.base_url))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "only the within-horizon change remains");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn forget_tombstones_a_subjects_payloads_but_keeps_the_audit_fact() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let id = record_a_rename(&st, "acme").await;

    // Erase the subject ("user:alice") who authored the rename.
    let app = app_for(&pg, admin("alice", "acme")).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/audit/forget", app.base_url))
        .json(&serde_json::json!({ "subject": "alice" }))
        .send()
        .await
        .expect("forget request");
    assert_eq!(resp.status(), 200, "admin may issue a forget");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["tombstoned"], 1, "the one authored row is tombstoned");

    // The audit fact survives (row still listed, op/resource intact) but the
    // before/after content is scrubbed.
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("{}/api/v1/audit", app.base_url))
        .send()
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "the audit row is preserved, not deleted");
    let row = &items[0];
    assert_eq!(row["resource"]["id"], id.to_string());
    assert_eq!(row["op"], "update", "the op (audit fact) is kept");
    assert!(row["before"].is_null(), "the before payload is scrubbed");
    assert!(row["after"].is_null(), "the after payload is scrubbed");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn forget_is_admin_gated() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;

    let app = app_for(&pg, writer("carol", "acme")).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/audit/forget", app.base_url))
        .json(&serde_json::json!({ "subject": "alice" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 403, "a non-admin cannot issue a forget");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn undo_restores_the_before_snapshot() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let id = record_a_rename(&st, "acme").await;

    let app = app_for(&pg, admin("alice", "acme")).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/undo", app.base_url))
        .send()
        .await
        .expect("undo request");
    assert_eq!(resp.status(), 200, "undo applies the inverse");

    // The rename is reversed: the dashboard is back to its pre-update name.
    let after_undo = dashboard::by_slug(&pg, "acme", "ops")
        .await
        .expect("read")
        .expect("dashboard still exists");
    assert_eq!(after_undo.id, id);
    assert_eq!(after_undo.name, "Ops", "undo restored the before snapshot");

    drop(app);
}
