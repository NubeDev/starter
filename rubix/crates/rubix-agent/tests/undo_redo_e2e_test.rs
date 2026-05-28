//! End-to-end coverage for the rockstar undo / redo flow against
//! a real Postgres pool. Pins three contracts the proposal v2
//! §3.4 calls out but no other test asserts as a single sequence:
//!
//!   1. **Undo round-trip.** Update a dashboard, undo, verify the
//!      live row's title + body revert to the original. Redo,
//!      verify they re-apply.
//!   2. **Redo cleared on new mutation.** With a redo stack
//!      primed, issue a *new* update through the same dispatcher,
//!      then call `rubix.undo.redo` — must fail with `NotFound`
//!      because the new mutation cleared the stack.
//!   3. **Cursor survives process restart.** After step (1)'s
//!      undo lands a redo entry, drop the cursor and service,
//!      rebuild a fresh `PgUndoCursor` + `UndoService` against
//!      the same pool, and call `redo_last`. Must succeed —
//!      proves the redo stack lives in Postgres, not in process
//!      memory.
//!
//! This is the "rockstar demo" the prior session's next-steps
//! list called for, distilled into a single
//! `#[ignore = "requires docker"]` integration test. Run via the
//! integration job, or locally with
//! `cargo test -p rubix-agent --test undo_redo_e2e_test -- --ignored`.

#![allow(clippy::too_many_lines)]

use std::sync::Arc;

use serde_json::json;
use starter_changelog_postgres::{
    migration_source as changelog_migration_source, PgChangeLog, PgChangeRecorder,
};
use starter_spi::changelog::{Actor, ChangeRecorder};
use starter_spi::error::Error;
use starter_spi::tool::Tool;
use starter_store_postgres::{migrate, testing::with_database};
use starter_undo::cursor_postgres::{migration_source as cursor_migration_source, PgUndoCursor};
use starter_undo::{redo_last, undo_last, ReversibleRegistry, UndoCursor, UndoService};

use rubix_spi::dashboard::DashboardStore;
use rubix_spi::dto::dashboard::create::CreateDashboardResponse;
use rubix_spi::dto::dashboard::update::UpdateDashboardResponse;
use rubix_store_postgres::{PgDashboardStore, DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE};

use rubix_tools::dashboard::create::DashboardCreateTool;
use rubix_tools::dashboard::store::{DashboardReversible, DASHBOARD_PAGE_KIND};
use rubix_tools::dashboard::update::DashboardUpdateTool;
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use starter_authz::StaticRegistry;
use starter_spi::authz::{Ownership, ResourceSpec};

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn undo_redo_round_trip_clear_on_mutation_and_cursor_survives_restart() {
    // ---------- container + schema -------------------------------
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE)
        .with_source(changelog_migration_source())
        .with_source(cursor_migration_source())
        .run()
        .await
        .expect("apply dashboards + changelog + undo cursor migrations");

    // ---------- shared wiring ------------------------------------
    let store: Arc<dyn DashboardStore> = Arc::new(PgDashboardStore::new(pool.clone()));
    let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool.clone()));
    let log = Arc::new(PgChangeLog::new(pool.clone()));
    let cursor: Arc<dyn UndoCursor> = Arc::new(PgUndoCursor::new(pool.clone()));

    let reversible = Arc::new(DashboardReversible::new(store.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let authz_registry = Arc::new(StaticRegistry::new());
    authz_registry.register_spec(ResourceSpec::from_static_tenant_scoped(
        DASHBOARD_PAGE_KIND,
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions`.",
    ));

    let actor = Actor::User {
        subject: "ada@x".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    // `with_cursor` is the production constructor: the dispatcher
    // clears the actor's redo stack on every successful reversible
    // mutation. Without this, step (2) below would silently pass.
    let create = UndoDispatcher::with_cursor(
        Arc::new(DashboardCreateTool::new(
            store.clone(),
            authz_registry.clone(),
        )),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
        cursor.clone(),
    );
    let update = UndoDispatcher::with_cursor(
        Arc::new(DashboardUpdateTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
        cursor.clone(),
    );

    let undo_service = Arc::new(UndoService::with_cursor(
        log.clone() as Arc<dyn starter_changelog::ChangeLog>,
        registry.clone(),
        cursor.clone(),
    ));

    // ---------- 1. create then update ----------------------------
    let create_out = create
        .invoke(json!({
            "tenant_id":       "tenant-a",
            "page_id":         "dashboard.ops",
            "owner_principal": "ada@x",
            "title":           "Ops v1",
            "tags":            ["custom"],
            "body_json":       { "ir_version": 5, "root": { "type": "page", "id": "v1", "children": [] } },
            "created_by":      "ada@x"
        }))
        .await
        .expect("create dispatch");
    let created: CreateDashboardResponse =
        serde_json::from_value(create_out).expect("CreateDashboardResponse decodes");
    let rev_v1 = created.revision_id.clone();

    let upd_out = update
        .invoke(json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": rev_v1,
            "title":                "Ops v2",
            "body_json":            { "ir_version": 5, "root": { "type": "page", "id": "v2", "children": [] } },
            "created_by":           "ada@x"
        }))
        .await
        .expect("update dispatch");
    let updated: UpdateDashboardResponse =
        serde_json::from_value(upd_out).expect("UpdateDashboardResponse decodes");
    assert!(updated.written);

    // Sanity: the live row carries the updated title.
    let live = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active")
        .expect("page is live");
    assert_eq!(live.title, "Ops v2");

    // ---------- 2. undo restores the prior revision --------------
    undo_last(&undo_service, &actor, None)
        .await
        .expect("undo.last");
    let live = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active")
        .expect("page still live");
    assert_eq!(
        live.title, "Ops v1",
        "undo must restore the prior title (rename round-trip, proposal §3.1)"
    );
    assert_eq!(
        live.body_json.get("root").and_then(|r| r.get("id")),
        Some(&json!("v1")),
        "undo must restore the prior body"
    );

    // The undo pushed the v2 group onto the cursor.
    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_some(),
        "undo populated the redo stack"
    );

    // ---------- 3. redo re-applies it ----------------------------
    redo_last(&undo_service, &actor, None)
        .await
        .expect("undo.redo");
    let live = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active")
        .expect("page still live");
    assert_eq!(live.title, "Ops v2", "redo must re-apply the update");
    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_none(),
        "redo popped the stack"
    );

    // ---------- 4. clear-on-mutation contract (§3.4) -------------
    // Undo once to re-prime the redo stack.
    undo_last(&undo_service, &actor, None)
        .await
        .expect("undo.last (second)");
    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_some(),
        "redo stack primed for the clear-on-mutation test"
    );

    // A new mutation by the same actor must clear the stack.
    let rev_after_undo = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active")
        .expect("page still live")
        .revision_id;
    let _ = update
        .invoke(json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": rev_after_undo,
            "title":                "Ops v3",
            "body_json":            { "ir_version": 5, "root": { "type": "page", "id": "v3", "children": [] } },
            "created_by":           "ada@x"
        }))
        .await
        .expect("third update dispatch");

    assert!(
        cursor.peek_redo(&actor).await.unwrap().is_none(),
        "new mutation must clear the redo stack (proposal §3.4)"
    );
    let redo_err = redo_last(&undo_service, &actor, None)
        .await
        .expect_err("redo after a fresh mutation must fail");
    assert!(
        matches!(redo_err, Error::NotFound { .. }),
        "redo after clear must return NotFound, got {redo_err:?}"
    );

    // ---------- 5. cursor survives "process restart" -------------
    // Re-undo to push a fresh entry onto the cursor.
    undo_last(&undo_service, &actor, None)
        .await
        .expect("undo to seed the cross-restart redo target");
    let peek_before_restart = cursor
        .peek_redo(&actor)
        .await
        .expect("peek before restart")
        .expect("redo target present");

    // Drop the cursor + service handles. The pool stays (a real
    // restart would re-`connect` against the same DSN; that is
    // equivalent at the SQL layer).
    drop(undo_service);
    drop(cursor);

    let restarted_cursor: Arc<dyn UndoCursor> = Arc::new(PgUndoCursor::new(pool.clone()));
    let peek_after_restart = restarted_cursor
        .peek_redo(&actor)
        .await
        .expect("peek after restart")
        .expect("redo target survived");
    assert_eq!(
        peek_before_restart.0, peek_after_restart.0,
        "redo stack top must survive a fresh PgUndoCursor handle (proves PG persistence)"
    );

    // And the fresh service can actually drive the redo through.
    let restarted_service = Arc::new(UndoService::with_cursor(
        Arc::new(PgChangeLog::new(pool.clone())) as Arc<dyn starter_changelog::ChangeLog>,
        registry.clone(),
        restarted_cursor.clone(),
    ));
    redo_last(&restarted_service, &actor, None)
        .await
        .expect("redo across restart");
    let live = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active")
        .expect("page live after restart-redo")
        .clone();
    assert_eq!(
        live.title, "Ops v3",
        "redo across restart must land the post-restart row at the same state as before the restart"
    );
}
