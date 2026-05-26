//! Goal 4 (clickhouse-ruler) integration coverage.
//!
//! Drives the `rubix.warehouse.retention.set` write verb through
//! the same [`UndoDispatcher`] seam the agent loop uses, asserts
//! that the underlying `ALTER TABLE … MODIFY TTL` ran (the
//! in-memory [`InMemoryWarehouseWriter`] stands in for the production
//! `ChClient`-backed impl), confirms that a snapshot row was
//! recorded in the changelog with `before = WarehouseRetentionSnapshot
//! { days: prior }`, then fires `rubix.undo.last` and asserts the
//! prior TTL is restored.
//!
//! Backing store note: the production CH-backed [`WarehouseWriter`] impl
//! lands in a follow-up phase that wires
//! `starter-store-warehouse::ChClient` to the same trait. Until
//! then the `InMemoryWarehouseWriter` fake is enough — the trait shape is
//! the contract, so the production swap is a one-line change in
//! the agent boot wiring and the assertions below stay green.
//! Equivalent end-to-end coverage through the `rubix-admin mcp`
//! transport will follow once the CH verbs are wired into
//! `boot::mcp::register::build_flow_registry`. See
//! [docs/design/warehouse-rules/](../../../docs/design/warehouse-rules/README.md)
//! for the snapshot shape and the `mart.create` undo data-loss
//! caveat.

use std::sync::Arc;

use serde_json::json;
use starter_changelog::{filter_for_actor, ChangeLog};
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::changelog::{Actor, Op};
use starter_spi::tool::Tool;
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::{ReversibleRegistry, UndoService};

use rubix_spi::dto::warehouse::retention_set::WarehouseRetentionSetResponse;
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;
use rubix_tools::warehouse::retention_set::WarehouseRetentionSetTool;
use rubix_tools::warehouse::store::{
    InMemoryWarehouseWriter, WarehouseRetentionReversible, WarehouseRetentionSnapshot,
    WarehouseWriter, WAREHOUSE_RETENTION_KIND,
};

#[tokio::test]
async fn retention_set_via_mcp_alters_ttl_records_snapshot_and_undo_restores_prior() {
    // ----- wiring --------------------------------------------------------
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");

    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(pool.clone()));

    // Production-shaped in-memory CH; pre-seed the table at 90d so
    // the verb has a real prior value to snapshot. The concrete
    // handle stays around for the test-helper accessors
    // (`retention(...)`); the dispatch path sees it through the
    // `WarehouseWriter` trait object.
    let in_mem: Arc<InMemoryWarehouseWriter> = Arc::new(InMemoryWarehouseWriter::new());
    in_mem.seed_retention("system_disk_history", 90);
    let writer: Arc<dyn WarehouseWriter> = in_mem.clone();

    let reversible = Arc::new(WarehouseRetentionReversible::new(writer.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let actor = Actor::User {
        subject: "ada@x".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    let retention = UndoDispatcher::new(
        Arc::new(WarehouseRetentionSetTool::new(writer.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );

    let undo_service = Arc::new(UndoService::new(log.clone(), registry.clone()));
    let undo_last = UndoLastTool::new(undo_service, actor_source);

    // ----- 1. set retention via the MCP-shaped dispatcher ---------------
    let out = retention
        .invoke(json!({"table_name": "system_disk_history", "days": 30}))
        .await
        .expect("retention.set dispatch succeeds");
    let resp: WarehouseRetentionSetResponse =
        serde_json::from_value(out).expect("response decodes");

    assert_eq!(
        resp.summary.code.as_str(),
        "rubix.warehouse.retention.set",
        "happy-path diagnostic",
    );
    assert_eq!(resp.prior_days, Some(90));
    assert_eq!(resp.days, 30);
    assert!(!resp.was_unchanged);

    // Assert the underlying ALTER actually ran — InMemoryWarehouseWriter
    // mirrors the production `ALTER TABLE … MODIFY TTL` effect.
    assert_eq!(
        in_mem.retention("system_disk_history"),
        Some(30),
        "writer reflects the new TTL after dispatch",
    );

    // Assert the snapshot row was recorded in the changelog with
    // before/after payloads that match the documented shape.
    let page = log
        .list(&filter_for_actor(&actor))
        .await
        .expect("list changelog rows");
    assert_eq!(
        page.items.len(),
        1,
        "retention.set dispatch recorded exactly one change",
    );
    let ch = &page.items[0];
    assert_eq!(ch.resource.kind, WAREHOUSE_RETENTION_KIND);
    assert_eq!(ch.resource.id.as_deref(), Some("system_disk_history"));
    assert!(matches!(ch.op, Op::Update));
    let before: WarehouseRetentionSnapshot =
        serde_json::from_value(ch.before.clone().expect("snapshot row has before payload"))
            .expect("before snapshot decodes");
    let after: WarehouseRetentionSnapshot =
        serde_json::from_value(ch.after.clone().expect("snapshot row has after payload"))
            .expect("after snapshot decodes");
    assert_eq!(before.days, Some(90), "snapshot row captured prior TTL");
    assert_eq!(after.days, Some(30), "snapshot row captured new TTL");

    // ----- 2. undo via rubix.undo.last restores the prior TTL -----------
    let undo_out = undo_last
        .invoke(json!({}))
        .await
        .expect("undo.last dispatch succeeds");
    assert!(
        undo_out.get("group_id").and_then(|v| v.as_str()).is_some(),
        "undo.last returns the undone group id; got {undo_out}",
    );
    assert_eq!(
        in_mem.retention("system_disk_history"),
        Some(90),
        "undo restored the prior 90d retention",
    );
}
