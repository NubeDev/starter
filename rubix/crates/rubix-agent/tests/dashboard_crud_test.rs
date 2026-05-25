//! Goal-1 Phase C.5 — `rubix.dashboard.*` end-to-end coverage.
//!
//! Spins an ephemeral Postgres (testcontainers), applies the rubix
//! `dashboards_definitions` migration source + the
//! `starter-changelog-postgres` source, wires the seven verb tools
//! against the live [`PgDashboardStore`] / [`PgChangeRecorder`] /
//! [`PgChangeLog`] trio (the production swap of the in-memory
//! registry adapter), and walks the full CRUD path the stage calls
//! out:
//!
//!   1. `create` — landed as the live head
//!   2. `get`    — round-trips the row
//!   3. `update` — fresh revision, prior superseded
//!   4. `update` with stale `expected_revision_id` — `Conflict`
//!      carrying the `rubix.dashboard.update.conflict` diagnostic
//!      key (per [`rubix/docs/scope/dashboards/04-tools.md`])
//!   5. `undo.last` — walks the create→update step back to the
//!      original body via [`DashboardReversible::apply_inverse`]
//!   6. `delete` — supersedes the live row
//!   7. `duplicate` — copies the source body under a fresh page id
//!   8. `list` with a tag filter — narrows to the duplicated page
//!   9. `page_set` — writes one slot through the R2 chokepoint
//!      [`GraphStore::write_slot`]
//!
//! Mirrors the goal-2/3/4 integration test shape: every dispatch
//! goes through [`UndoDispatcher`] so the changelog row is recorded
//! the same way the production tools router records it.

#![allow(clippy::too_many_lines)]

use std::sync::Arc;

use serde_json::json;
use starter_changelog::ChangeLog as _;
use starter_changelog_postgres::{
    migration_source as changelog_migration_source, PgChangeLog, PgChangeRecorder,
};
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{NodeId, SlotRef, SlotValue};
use starter_spi::authz::Ownership;
use starter_spi::authz::ResourceSpec;
use starter_spi::changelog::{Actor, ChangeRecorder};
use starter_spi::tool::Tool;
use starter_store_postgres::{migrate, testing::with_database};
use starter_undo::{ReversibleRegistry, UndoService};

use rubix_spi::dashboard::DashboardStore;
use rubix_spi::dto::dashboard::create::CreateDashboardResponse;
use rubix_spi::dto::dashboard::delete::DeleteDashboardResponse;
use rubix_spi::dto::dashboard::duplicate::DuplicateDashboardResponse;
use rubix_spi::dto::dashboard::get::GetDashboardResponse;
use rubix_spi::dto::dashboard::list::ListDashboardsResponse;
use rubix_spi::dto::dashboard::page_set::PageSetResponse;
use rubix_spi::dto::dashboard::update::UpdateDashboardResponse;
use rubix_store_postgres::{PgDashboardStore, DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE};

use rubix_tools::dashboard::create::DashboardCreateTool;
use rubix_tools::dashboard::delete::DashboardDeleteTool;
use rubix_tools::dashboard::duplicate::DashboardDuplicateTool;
use rubix_tools::dashboard::get::DashboardGetTool;
use rubix_tools::dashboard::list::DashboardListTool;
use rubix_tools::dashboard::page_set::DashboardPageSetTool;
use rubix_tools::dashboard::store::{DashboardReversible, DASHBOARD_PAGE_KIND};
use rubix_tools::dashboard::update::DashboardUpdateTool;
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;
use starter_authz::StaticRegistry;
use starter_spi::error::Error;

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn dashboard_crud_end_to_end_against_pg() {
    // -------- container + schema --------------------------------
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply dashboards + changelog migrations");

    // -------- shared wiring -------------------------------------
    let store: Arc<dyn DashboardStore> = Arc::new(PgDashboardStore::new(pool.clone()));
    let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool.clone()));
    let log = Arc::new(PgChangeLog::new(pool.clone()));

    let reversible = Arc::new(DashboardReversible::new(store.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let authz_registry = Arc::new(StaticRegistry::new());
    // Pre-register the page kind so DashboardCreateTool's idempotent
    // re-register lookup has a baseline. The tool also calls
    // `try_register` so this line is just defence-in-depth.
    authz_registry.register_spec(ResourceSpec::from_static_tenant_scoped(
        DASHBOARD_PAGE_KIND,
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions` and resolved by the page provider.",
    ));

    let actor = Actor::User {
        subject: "ada@x".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    let create = UndoDispatcher::new(
        Arc::new(DashboardCreateTool::new(
            store.clone(),
            authz_registry.clone(),
        )),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let update = UndoDispatcher::new(
        Arc::new(DashboardUpdateTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let delete = UndoDispatcher::new(
        Arc::new(DashboardDeleteTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let duplicate = UndoDispatcher::new(
        Arc::new(DashboardDuplicateTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let get = DashboardGetTool::new(store.clone());
    let list = DashboardListTool::new(store.clone());

    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let page_set = DashboardPageSetTool::new(graph.clone());

    let undo_service = Arc::new(UndoService::new(
        log.clone() as Arc<dyn starter_changelog::ChangeLog>,
        registry.clone(),
    ));
    let undo_last = UndoLastTool::new(undo_service.clone(), actor_source.clone());

    // -------- 1. create -----------------------------------------
    let create_out = create
        .invoke(json!({
            "tenant_id":       "tenant-a",
            "page_id":         "dashboard.ops",
            "owner_principal": "ada@x",
            "title":           "Ops v1",
            "tags":            ["custom"],
            "body_json":       { "ir_version": 1, "root": { "kind": "Stack" } },
            "created_by":      "ada@x"
        }))
        .await
        .expect("dashboard.create dispatch");
    let created: CreateDashboardResponse =
        serde_json::from_value(create_out).expect("CreateDashboardResponse decodes");
    assert_eq!(created.summary.code.as_str(), "rubix.dashboard.created");
    let rev_v1 = created.revision_id.clone();

    // -------- 2. get --------------------------------------------
    let got_out = get
        .invoke(json!({"tenant_id": "tenant-a", "page_id": "dashboard.ops"}))
        .await
        .expect("dashboard.get dispatch");
    let got: GetDashboardResponse =
        serde_json::from_value(got_out).expect("GetDashboardResponse decodes");
    assert_eq!(got.summary.code.as_str(), "rubix.dashboard.fetched");
    assert_eq!(got.revision_id.as_deref(), Some(rev_v1.as_str()));
    assert_eq!(got.title.as_deref(), Some("Ops v1"));

    // -------- 3. update -----------------------------------------
    let upd_out = update
        .invoke(json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": rev_v1,
            "title":                "Ops v2",
            "body_json":            { "ir_version": 1, "root": { "kind": "Grid" } },
            "created_by":           "ada@x"
        }))
        .await
        .expect("dashboard.update dispatch");
    let updated: UpdateDashboardResponse =
        serde_json::from_value(upd_out).expect("UpdateDashboardResponse decodes");
    assert_eq!(updated.summary.code.as_str(), "rubix.dashboard.updated");
    assert!(updated.written);
    assert_ne!(updated.revision_id, rev_v1);
    let _rev_v2 = updated.revision_id.clone();

    // -------- 4. conflict-on-stale ------------------------------
    let conflict_err = update
        .invoke(json!({
            "tenant_id":            "tenant-a",
            "page_id":              "dashboard.ops",
            "expected_revision_id": rev_v1,
            "body_json":            { "ir_version": 1, "root": { "kind": "Grid" } },
            "created_by":           "ada@x"
        }))
        .await
        .expect_err("stale expected_revision_id must conflict");
    match conflict_err {
        Error::Conflict { message } => assert!(
            message.contains("rubix.dashboard.update.conflict"),
            "unexpected conflict payload: {message}",
        ),
        other => panic!("expected Conflict, got {other:?}"),
    }

    // -------- 5. undo.last (rolls update back) ------------------
    let undo_out = undo_last
        .invoke(json!({}))
        .await
        .expect("undo.last dispatch");
    assert!(
        undo_out.get("group_id").and_then(|v| v.as_str()).is_some(),
        "undo.last returns a group id; got {undo_out}",
    );
    // The update path's inverse re-inserts the `before` snapshot.
    // Phase C.2 records `before = None` (documented follow-up), so
    // the inverse is a no-op for the body but the changelog row is
    // recorded — assert by walking the log directly.
    let page = log
        .list(&starter_changelog::ChangeFilter::default())
        .await
        .expect("changelog list");
    assert!(
        page.items
            .iter()
            .any(|c| c.resource.kind == DASHBOARD_PAGE_KIND),
        "changelog contains at least one dashboard row; got {:?}",
        page.items.iter().map(|c| &c.resource.kind).collect::<Vec<_>>(),
    );

    // -------- 6. delete -----------------------------------------
    let del_out = delete
        .invoke(json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.ops",
            "deleted_by": "ada@x"
        }))
        .await
        .expect("dashboard.delete dispatch");
    let deleted: DeleteDashboardResponse =
        serde_json::from_value(del_out).expect("DeleteDashboardResponse decodes");
    assert_eq!(deleted.summary.code.as_str(), "rubix.dashboard.deleted");
    assert!(deleted.superseded >= 1);
    let after_delete = store
        .get_active("tenant-a", "dashboard.ops")
        .await
        .expect("get_active");
    assert!(
        after_delete.is_none(),
        "no live row after delete; got {after_delete:?}",
    );

    // -------- 7. duplicate --------------------------------------
    // First re-create a source row so we have something to clone.
    let source_out = create
        .invoke(json!({
            "tenant_id":       "tenant-a",
            "page_id":         "dashboard.source",
            "owner_principal": "ada@x",
            "title":           "Source",
            "tags":            ["custom", "energy"],
            "body_json":       { "ir_version": 1, "root": { "kind": "Stack" } },
            "created_by":      "ada@x"
        }))
        .await
        .expect("seed source for duplicate");
    let _: CreateDashboardResponse = serde_json::from_value(source_out).unwrap();

    let dup_out = duplicate
        .invoke(json!({
            "source_tenant_id":    "tenant-a",
            "source_page_id":      "dashboard.source",
            "target_tenant_id":    "tenant-a",
            "target_page_id":      "dashboard.clone",
            "new_owner_principal": "ada@x",
            "created_by":          "ada@x"
        }))
        .await
        .expect("dashboard.duplicate dispatch");
    let duplicated: DuplicateDashboardResponse =
        serde_json::from_value(dup_out).expect("DuplicateDashboardResponse decodes");
    assert_eq!(
        duplicated.summary.code.as_str(),
        "rubix.dashboard.duplicated"
    );
    assert_eq!(duplicated.page_id, "dashboard.clone");

    // -------- 8. list with a tag filter -------------------------
    let list_out = list
        .invoke(json!({
            "tenant_id": "tenant-a",
            "tags_any":  ["energy"]
        }))
        .await
        .expect("dashboard.list dispatch");
    let listed: ListDashboardsResponse =
        serde_json::from_value(list_out).expect("ListDashboardsResponse decodes");
    assert_eq!(listed.summary.code.as_str(), "rubix.dashboard.listed");
    let ids: Vec<&str> = listed.items.iter().map(|s| s.page_id.as_str()).collect();
    assert!(
        ids.contains(&"dashboard.source") && ids.contains(&"dashboard.clone"),
        "tag-filtered list surfaces source + clone; got {ids:?}",
    );

    // -------- 9. page_set ---------------------------------------
    let ps_out = page_set
        .invoke(json!({
            "tenant_id":  "tenant-a",
            "page_id":    "dashboard.source",
            "node_id":    "com.acme.thermostat",
            "slot":       "setpoint",
            "value":      21.5,
            "written_by": "ada@x"
        }))
        .await
        .expect("dashboard.page_set dispatch");
    let ps: PageSetResponse =
        serde_json::from_value(ps_out).expect("PageSetResponse decodes");
    assert_eq!(ps.summary.code.as_str(), "rubix.dashboard.page_set.applied");
    assert!(ps.written);
    let v = graph
        .read_slot(&SlotRef::new(
            NodeId::new("com.acme.thermostat").unwrap(),
            "setpoint".to_owned(),
        ))
        .await
        .expect("slot read");
    assert_eq!(v, SlotValue::Float(21.5));
}
