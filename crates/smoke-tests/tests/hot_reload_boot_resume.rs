//! HR5 smoke — "Boot resumes to last-known-good"
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests").
//!
//! Across a process restart (simulated by dropping the
//! `DefinitionManager` + `Pool` and reopening the same file-
//! backed SQLite database), the new manager's `boot_resume()`
//! mounts the head revision from the persistent FlowStore and
//! emits `Mounted`. A run started against the post-resume
//! active topology fires the nodes encoded in the persisted
//! head — proving the engine resumed without an explicit
//! republish.
//!
//! This is the file-backed end-to-end variant of the manager-
//! level `hr4_boot_resume_mounts_known_flows` unit test
//! (`crates/starter-flow/src/definition/manager.rs`): same
//! contract, exercised through the real sqlite chokepoint with
//! the engine + runner attached.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::timeout;

use starter_flow::definition::DefinitionManager;
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::{FlowRunner, InMemoryRunStore, RunSpec, RunStore};
use starter_flow::state::RunStatus;
use starter_flow_spi::definition::{DefinitionSource, FlowDefinitionEvent};
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_store_sqlite::flow::{SqliteFlowStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, pool::connect, Pool};

/// Tap kind: echoes input.in to out and records invocations.
struct Tap {
    kind: KindId,
    calls: Arc<AtomicU64>,
}
impl Tap {
    fn arc(s: &str) -> (Arc<Self>, Arc<AtomicU64>) {
        let calls = Arc::new(AtomicU64::new(0));
        (
            Arc::new(Self {
                kind: KindId::new(s).unwrap(),
                calls: Arc::clone(&calls),
            }),
            calls,
        )
    }
}
#[async_trait]
impl NodeBehavior for Tap {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let n = match input.get("in") {
            Some(SlotValue::Int(n)) => *n,
            _ => 0,
        };
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), SlotValue::Int(n));
        Ok(out)
    }
}

fn slot(node: &str, name: &str) -> SlotRef {
    SlotRef::new(NodeId::new(node).unwrap(), name)
}

async fn open_file_pool(path: &std::path::Path) -> Pool {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = connect(&url).await.expect("connect file sqlite");
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    pool
}

#[tokio::test]
async fn hot_reload_boot_resume_mounts_last_known_good_head() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_path = tmp.path().join("flow.db");
    let flow = FlowId::new("examples.smoke.resume").unwrap();

    // ---------- Process 1: publish a head revision and drop. ----------
    {
        let pool = open_file_pool(&db_path).await;
        let flow_store = Arc::new(SqliteFlowStore::new(pool));
        let kinds = Arc::new(NodeKindRegistry::new());
        let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let mgr = Arc::new(DefinitionManager::with_graph(
            flow_store,
            Arc::clone(&kinds),
            Arc::clone(&graph),
        ));
        // Register the kinds the body references so resolve passes.
        let (tap, _calls) = Tap::arc("com.acme.smoke.tap");
        kinds.register(tap).await.expect("register tap");

        let body = serde_json::json!({
            "flow_id": "examples.smoke.resume",
            "apply_policy": "drain",
            "nodes": [
                {"id": "smoke.head", "kind": "com.acme.smoke.tap",
                 "triggers": ["in"]},
                {"id": "smoke.tail", "kind": "com.acme.smoke.tap",
                 "triggers": ["in"]}
            ],
            "links": [{"from": "smoke.head.out", "to": "smoke.tail.in"}]
        });
        mgr.publish(flow.clone(), body, DefinitionSource::Api)
            .await
            .expect("v1 publish");
        assert_eq!(
            mgr.active_topologies().len().await,
            1,
            "publish mounts the head in-process"
        );

        // Drop the manager and pool, simulating a process exit.
        drop(mgr);
    }

    // ---------- Process 2: reopen, boot_resume, run against resumed head. ----------
    let pool = open_file_pool(&db_path).await;
    let flow_store = Arc::new(SqliteFlowStore::new(pool));
    let kinds = Arc::new(NodeKindRegistry::new());
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let mgr = Arc::new(DefinitionManager::with_graph(
        flow_store,
        Arc::clone(&kinds),
        Arc::clone(&graph),
    ));
    let engine = Engine::new(Arc::clone(&graph))
        .with_node_kinds(Arc::clone(&kinds))
        .with_definition_manager(Arc::clone(&mgr));
    let (tap, tap_calls) = Tap::arc("com.acme.smoke.tap");
    engine.register_kind(tap).await.expect("register tap");

    // Active topologies must start empty (fresh process).
    assert_eq!(
        mgr.active_topologies().len().await,
        0,
        "fresh manager has nothing mounted before boot_resume"
    );

    let mut def_rx = mgr.subscribe();
    let report = mgr.boot_resume().await.expect("boot_resume");
    assert_eq!(report.mounted, 1, "head revision must remount");
    assert_eq!(report.failed, 0);
    assert_eq!(report.skipped, 0);
    assert_eq!(
        mgr.active_topologies().len().await,
        1,
        "active topology populated from persisted head"
    );

    // Bus must carry a Mounted event for the resumed flow.
    let mut saw_mounted = false;
    for _ in 0..8 {
        match timeout(Duration::from_millis(120), def_rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::Mounted { flow: f, .. })) if f == flow => {
                saw_mounted = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_mounted,
        "boot_resume must emit Mounted for the persisted flow"
    );

    // ---------- Run a flow against the resumed topology. ----------
    let topology = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("mounted post-resume")
        .load();
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let runner = FlowRunner::new(Arc::clone(&graph), Arc::clone(&run_store));
    let spec = RunSpec::new(
        flow.clone(),
        FlowRevisionId::new(),
        Arc::clone(&topology),
        vec![(slot("smoke.head", "in"), SlotValue::Int(7))],
        vec![slot("smoke.tail", "out")],
    );
    let handle = runner.start(spec, SlotMap::new()).await.expect("run start");
    let status = timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("run did not complete")
        .expect("coordinator panicked");
    assert_eq!(status, RunStatus::Completed);
    assert_eq!(
        tap_calls.load(Ordering::SeqCst),
        2,
        "head + tail both fire against the resumed topology"
    );
}
