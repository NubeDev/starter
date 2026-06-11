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
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        datasource_kinds: Arc::new(nexus_api::datasource_kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(ENVELOPE_KEY, 1).unwrap(),
        ),
        query_cache: nexus_api::cache::CacheConfig::default().build(),
        quotas: nexus_api::quota::TenantQuotas::new(nexus_api::quota::QuotaConfig::default()),
        rate_limiter: nexus_api::ratelimit::TenantRateLimiter::new(
            nexus_api::ratelimit::RateLimitConfig::default(),
        ),
        canary: Default::default(),
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
        folder_id: None,
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

/// Regression for the reported bug: before panels recorded their own changes,
/// an Undo issued after editing a dashboard fell through to the dashboard's
/// `Create` row and **deleted the whole dashboard**. With panel recording, undo
/// of an add-panel must delete only that panel and leave the dashboard intact.
#[tokio::test]
#[ignore = "requires docker"]
async fn undo_of_add_panel_removes_the_panel_not_the_dashboard() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let dash = dashboard::insert(&st.metadata, "acme", &new_dashboard("ops", "Ops"))
        .await
        .expect("create dashboard");

    let app = app_for(&pg, admin("alice", "acme")).await;
    let http = reqwest::Client::new();

    // Add a panel through the real handler (records a panel Create).
    let created: serde_json::Value = http
        .post(format!("{}/api/v1/dashboards/ops/panels", app.base_url))
        .json(&serde_json::json!({ "title": "Energy", "sql": "SELECT 1" }))
        .send()
        .await
        .expect("add panel")
        .json()
        .await
        .expect("json");
    let panel_id = created["id"].as_str().expect("panel id").to_string();
    assert_eq!(
        dashboard::panel::list_for_dashboard(&pg, "acme", dash.id)
            .await
            .expect("list")
            .len(),
        1,
        "panel was added",
    );

    // Undo: the most recent recorded group is the panel Create, so undo deletes
    // the panel — NOT the dashboard (the bug).
    let resp = http
        .post(format!("{}/api/v1/undo", app.base_url))
        .send()
        .await
        .expect("undo");
    assert_eq!(resp.status(), 200, "undo applies");

    assert!(
        dashboard::by_slug(&pg, "acme", "ops")
            .await
            .expect("read")
            .is_some(),
        "the dashboard must survive an undo of a panel add",
    );
    assert!(
        dashboard::panel::get(&pg, "acme", Uuid::parse_str(&panel_id).unwrap())
            .await
            .expect("read panel")
            .is_none(),
        "undo removed the added panel",
    );

    drop(app);
}

/// Undo of a panel *update* restores the panel's prior fields; redo re-applies
/// the edit. Proves the snapshot round-trip through the real HTTP path.
#[tokio::test]
#[ignore = "requires docker"]
async fn undo_redo_of_panel_update_round_trips() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let dash = dashboard::insert(&st.metadata, "acme", &new_dashboard("ops", "Ops"))
        .await
        .expect("create dashboard");
    let panel = dashboard::panel::insert(
        &st.metadata,
        "acme",
        &nexus_store::dashboard::NewPanel {
            dashboard_id: dash.id,
            datasource_id: None,
            title: "Before".into(),
            sql: "SELECT 1".into(),
            viz: "table".into(),
            layout: serde_json::json!({}),
        },
    )
    .await
    .expect("insert panel");

    let app = app_for(&pg, admin("alice", "acme")).await;
    let http = reqwest::Client::new();

    // Edit the panel title through the handler (records a panel Update with a
    // non-null `before`).
    http.patch(format!("{}/api/v1/panels/{}", app.base_url, panel.id))
        .json(&serde_json::json!({ "title": "After" }))
        .send()
        .await
        .expect("update panel");
    let read = |pg: sqlx::PgPool, id: Uuid| async move {
        dashboard::panel::get(&pg, "acme", id)
            .await
            .expect("read")
            .expect("panel exists")
            .title
    };
    assert_eq!(read(pg.clone(), panel.id).await, "After", "edit applied");

    // Undo → back to "Before"; redo → "After" again.
    http.post(format!("{}/api/v1/undo", app.base_url))
        .send()
        .await
        .expect("undo");
    assert_eq!(read(pg.clone(), panel.id).await, "Before", "undo restored before");

    http.post(format!("{}/api/v1/redo", app.base_url))
        .send()
        .await
        .expect("redo");
    assert_eq!(read(pg.clone(), panel.id).await, "After", "redo re-applied after");

    drop(app);
}

/// Undo of a panel *delete* resurrects the panel under its original id (the
/// dashboard layout addresses panels by id, so a fresh id would orphan it).
#[tokio::test]
#[ignore = "requires docker"]
async fn undo_of_panel_delete_resurrects_under_original_id() {
    let (admin_pool, _guard) = with_database().await;
    let pg = runtime_pool(admin_pool.sqlx()).await;
    let st = state(&pg);
    let dash = dashboard::insert(&st.metadata, "acme", &new_dashboard("ops", "Ops"))
        .await
        .expect("create dashboard");
    let panel = dashboard::panel::insert(
        &st.metadata,
        "acme",
        &nexus_store::dashboard::NewPanel {
            dashboard_id: dash.id,
            datasource_id: None,
            title: "Keep me".into(),
            sql: "SELECT 1".into(),
            viz: "table".into(),
            layout: serde_json::json!({ "x": 1 }),
        },
    )
    .await
    .expect("insert panel");

    let app = app_for(&pg, admin("alice", "acme")).await;
    let http = reqwest::Client::new();

    http.delete(format!("{}/api/v1/panels/{}", app.base_url, panel.id))
        .send()
        .await
        .expect("delete panel");
    assert!(
        dashboard::panel::get(&pg, "acme", panel.id)
            .await
            .expect("read")
            .is_none(),
        "panel deleted",
    );

    http.post(format!("{}/api/v1/undo", app.base_url))
        .send()
        .await
        .expect("undo");
    let resurrected = dashboard::panel::get(&pg, "acme", panel.id)
        .await
        .expect("read")
        .expect("panel resurrected under original id");
    assert_eq!(resurrected.id, panel.id, "same id, so layout stays valid");
    assert_eq!(resurrected.title, "Keep me");
    assert_eq!(resurrected.layout, serde_json::json!({ "x": 1 }));

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
