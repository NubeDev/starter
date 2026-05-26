//! Goal 3 (flow-programmer) integration coverage.
//!
//! Drives the `rubix.flow_ops.duplicate` write verb through the
//! same [`UndoDispatcher`] seam the agent loop uses, asserts that a
//! new revision row landed in the backing [`FlowDefStore`] under
//! the target `flow_id`, fires `rubix.flow_ops.list` and asserts
//! both the source (`com.rubix.scheduled-system-check`) and the
//! freshly-duplicated target surface, then fires `rubix.undo.last`
//! and asserts the new revision is superseded — `rubix.flow_ops.list`
//! reverts to surfacing the source alone.
//!
//! Backing store note: the PG-backed `FlowDefStore` impl lands in
//! a follow-up phase (see
//! [docs/design/flow-programmer/](../../../docs/design/flow-programmer/README.md)).
//! Until then the `InMemoryFlowDefStore` stands in — the trait
//! shape is the contract, so the production swap is a one-line
//! change in the agent boot wiring and the assertions below stay
//! green. Equivalent end-to-end coverage through the
//! `rubix-admin mcp` transport will follow once the `flow_ops`
//! verbs are wired into `boot::mcp::register::build_flow_registry`.

use std::sync::Arc;

use serde_json::json;
use starter_changelog::ChangeLog;
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_spi::changelog::Actor;
use starter_spi::tool::Tool;
use starter_store_sqlite::{migrate, testing::ephemeral};
use starter_undo::{ReversibleRegistry, UndoService};

use rubix_spi::dto::flow_ops::duplicate::FlowDuplicateResponse;
use rubix_spi::dto::flow_ops::list::FlowListResponse;
use rubix_tools::flow_ops::duplicate::FlowDuplicateTool;
use rubix_tools::flow_ops::list::FlowListTool;
use rubix_tools::flow_ops::store::{FlowDefReversible, FlowDefStore, InMemoryFlowDefStore};
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;

const SCHED_YAML: &str = "id: com.rubix.scheduled-system-check\ntrigger: explicit\nnodes:\n  - id: check\n    kind: ai-agent\n    config: {}\nlinks: []\n";

#[tokio::test]
async fn duplicate_via_mcp_writes_revision_lists_both_and_undo_reverts() {
    // ----- wiring --------------------------------------------------------
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");

    let recorder = Arc::new(SqliteChangeRecorder::new(pool.clone()));
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(pool.clone()));

    // Seed the source flow as a live revision so the duplicate verb
    // has something to read. Stands in for the boot-time seed of
    // `rubix_flows::BUNDLED` into `flows_definitions`.
    let store_concrete: Arc<InMemoryFlowDefStore> = Arc::new(InMemoryFlowDefStore::new());
    store_concrete
        .insert_revision("com.rubix.scheduled-system-check", SCHED_YAML, 1)
        .await
        .expect("seed source flow");
    let store: Arc<dyn FlowDefStore> = store_concrete.clone();

    let reversible = Arc::new(FlowDefReversible::new(store.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let actor = Actor::User {
        subject: "ada@x".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    let duplicate = UndoDispatcher::new(
        Arc::new(FlowDuplicateTool::new(store.clone())),
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    );
    let list = FlowListTool::new(store.clone());

    let undo_service = Arc::new(UndoService::new(log.clone(), registry.clone()));
    let undo_last = UndoLastTool::new(undo_service, actor_source);

    // ----- 1. duplicate via the MCP-shaped dispatcher -------------------
    let out = duplicate
        .invoke(json!({
            "source_flow_id": "com.rubix.scheduled-system-check",
            "target_flow_id": "com.rubix.scheduled-system-check-copy",
        }))
        .await
        .expect("flow_ops.duplicate dispatch succeeds");
    let resp: FlowDuplicateResponse =
        serde_json::from_value(out).expect("FlowDuplicateResponse decodes");

    assert_eq!(
        resp.summary.code.as_str(),
        "rubix.flow.duplicated",
        "duplicate emits rubix.flow.duplicated",
    );
    assert_eq!(resp.target_flow_id, "com.rubix.scheduled-system-check-copy");

    // Assert the new revision row landed in the backing store and is
    // live (not superseded).
    let new_rev = store_concrete
        .get(&resp.revision_id)
        .expect("revision row persisted");
    assert_eq!(
        new_rev.flow_id, "com.rubix.scheduled-system-check-copy",
        "new row carries the target flow_id",
    );
    assert!(
        new_rev.superseded_at_ms.is_none(),
        "new revision is live immediately after duplicate",
    );
    assert!(
        new_rev
            .body_yaml
            .contains("id: com.rubix.scheduled-system-check-copy"),
        "body_yaml rewrote `id:` to the target",
    );

    // ----- 2. tools/list surfaces both flows ----------------------------
    let listed = list
        .invoke(json!({}))
        .await
        .expect("flow_ops.list dispatch succeeds");
    let listed: FlowListResponse =
        serde_json::from_value(listed).expect("FlowListResponse decodes");
    assert_eq!(listed.summary.code.as_str(), "rubix.flow.listed");
    let ids: Vec<&str> = listed.flows.iter().map(|f| f.flow_id.as_str()).collect();
    assert!(
        ids.contains(&"com.rubix.scheduled-system-check"),
        "list surfaces the source; got {ids:?}",
    );
    assert!(
        ids.contains(&"com.rubix.scheduled-system-check-copy"),
        "list surfaces the duplicated target; got {ids:?}",
    );
    assert_eq!(listed.count, 2);

    // ----- 3. undo via rubix.undo.last walks the duplicate back ---------
    let undo_out = undo_last
        .invoke(json!({}))
        .await
        .expect("undo.last dispatch succeeds");
    assert!(
        undo_out.get("group_id").and_then(|v| v.as_str()).is_some(),
        "undo.last returns the undone group id; got {undo_out}",
    );

    // The freshly-duplicated revision is superseded; since the
    // target had no prior live revision (per `FlowDefChange::
    // prior_revision_id = None` on the duplicate path), the target
    // flow_id is left with no live revision at all.
    let after = store_concrete
        .get(&resp.revision_id)
        .expect("revision row still present after undo");
    assert!(
        after.superseded_at_ms.is_some(),
        "undo of duplicate marked the new revision superseded; got {:?}",
        after.superseded_at_ms,
    );

    // ----- 4. tools/list reverts to the source alone --------------------
    let listed = list
        .invoke(json!({}))
        .await
        .expect("flow_ops.list dispatch succeeds after undo");
    let listed: FlowListResponse =
        serde_json::from_value(listed).expect("FlowListResponse decodes");
    let ids: Vec<&str> = listed.flows.iter().map(|f| f.flow_id.as_str()).collect();
    assert_eq!(
        listed.count, 1,
        "exactly one live revision after undo; got {ids:?}",
    );
    assert_eq!(
        ids,
        vec!["com.rubix.scheduled-system-check"],
        "list reverts to the source alone after undo",
    );
}
