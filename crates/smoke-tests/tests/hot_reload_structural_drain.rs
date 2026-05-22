//! HR4 structural-drain smoke test
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests" — "Structural
//! edit drains in-flight runs").
//!
//! A flow with `apply_policy: drain` has run R1 in flight (mid-
//! `agent`). An operator publishes a draft that adds a downstream
//! node. R1 continues against the OLD topology snapshot it was
//! started with (the new node never fires for R1). A subsequent
//! run R2 sees the new topology and the new node DOES fire.
//!
//! Mechanics: each `FlowRunner::start` clones the
//! `Arc<FlowTopology>` from the [`ActiveTopologies`] into the
//! per-run snapshot. A structural publish swaps the `ArcSwap`
//! pointer to a new `Arc<FlowTopology>` but the in-flight run
//! holds its old `Arc` — so a Drain-policy publish that swaps
//! the active pointer cannot leak the new node into the
//! in-flight run.
//!
//! Wired through the real public surface (Engine +
//! DefinitionManager + SqliteFlowStore + FlowRunner +
//! InMemoryRunStore). The blocking agent kind uses a Tokio
//! `Notify` so the test can hold R1 mid-invoke while it publishes
//! the structural edit.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;
use tokio::time::timeout;

use starter_flow::definition::{DefinitionManager, PublishOutcome};
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::{FlowRunner, InMemoryRunStore, RunSpec, RunStore};
use starter_flow::state::RunStatus;
use starter_flow_spi::definition::{DefinitionSource, EditKindTag, FlowDefinitionEvent};
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotRef, SlotValue,
};
use starter_flow_spi::Cancel;
use starter_store_sqlite::flow::{SqliteFlowStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral};

/// Agent kind: blocks on `gate.notified()` before returning, so the
/// test can pause R1 mid-invoke. Records its invocation count.
struct BlockingAgent {
    kind: KindId,
    gate: Arc<Notify>,
    calls: Arc<AtomicU64>,
}
impl BlockingAgent {
    fn arc(s: &str, gate: Arc<Notify>) -> (Arc<Self>, Arc<AtomicU64>) {
        let calls = Arc::new(AtomicU64::new(0));
        (
            Arc::new(Self {
                kind: KindId::new(s).unwrap(),
                gate,
                calls: Arc::clone(&calls),
            }),
            calls,
        )
    }
}
#[async_trait]
impl NodeBehavior for BlockingAgent {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Wait until the test releases the gate.
        self.gate.notified().await;
        let n = match input.get("in") {
            Some(SlotValue::Int(n)) => *n,
            _ => 0,
        };
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), SlotValue::Int(n));
        Ok(out)
    }
}

/// Tap kind: records invocations, copies input to output so the
/// chain terminates at the test's chosen terminal slot.
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
    use starter_flow_spi::node::NodeId;
    SlotRef::new(NodeId::new(node).unwrap(), name)
}

#[tokio::test]
async fn hot_reload_structural_edit_drains_in_flight_runs() {
    // ---------- engine + manager + run plumbing ----------
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
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

    // Wire kinds with the gate the test holds.
    let gate = Arc::new(Notify::new());
    let (agent, agent_calls) = BlockingAgent::arc("com.acme.smoke.agent", Arc::clone(&gate));
    let (log, log_calls) = Tap::arc("com.acme.smoke.log");
    let (extra, extra_calls) = Tap::arc("com.acme.smoke.extra");
    engine.register_kind(agent).await.expect("register agent");
    engine.register_kind(log).await.expect("register log");
    engine.register_kind(extra).await.expect("register extra");

    // ---------- v1 publish: agent -> log, drain policy ----------
    let flow = FlowId::new("examples.smoke.drain").unwrap();
    let v1 = serde_json::json!({
        "flow_id": "examples.smoke.drain",
        "apply_policy": "drain",
        "nodes": [
            {"id": "smoke.agent", "kind": "com.acme.smoke.agent",
             "triggers": ["in"]},
            {"id": "smoke.log", "kind": "com.acme.smoke.log",
             "triggers": ["in"]}
        ],
        "links": [{"from": "smoke.agent.out", "to": "smoke.log.in"}]
    });
    let _ = mgr
        .publish(flow.clone(), v1, DefinitionSource::Api)
        .await
        .expect("v1 publish");

    let v1_topology = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("mounted")
        .load();

    // ---------- start R1 against the v1 snapshot ----------
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let runner = FlowRunner::new(Arc::clone(&graph), Arc::clone(&run_store));
    let spec1 = RunSpec::new(
        flow.clone(),
        FlowRevisionId::new(),
        Arc::clone(&v1_topology),
        vec![(slot("smoke.agent", "in"), SlotValue::Int(1))],
        vec![slot("smoke.log", "out")],
    );
    let r1 = runner.start(spec1, SlotMap::new()).await.expect("R1 start");

    // Wait until the agent has actually entered invoke (so R1 is
    // genuinely mid-flight at the moment of the structural publish).
    for _ in 0..50 {
        if agent_calls.load(Ordering::SeqCst) >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        agent_calls.load(Ordering::SeqCst),
        1,
        "R1's agent must be mid-invoke before the structural publish"
    );

    // ---------- structural publish: add `extra` downstream of log ----------
    let mut def_rx = mgr.subscribe();
    let v2 = serde_json::json!({
        "flow_id": "examples.smoke.drain",
        "apply_policy": "drain",
        "nodes": [
            {"id": "smoke.agent", "kind": "com.acme.smoke.agent",
             "triggers": ["in"]},
            {"id": "smoke.log", "kind": "com.acme.smoke.log",
             "triggers": ["in"]},
            {"id": "smoke.extra", "kind": "com.acme.smoke.extra",
             "triggers": ["in"]}
        ],
        "links": [
            {"from": "smoke.agent.out", "to": "smoke.log.in"},
            {"from": "smoke.log.out", "to": "smoke.extra.in"}
        ]
    });
    let out = mgr
        .publish(flow.clone(), v2, DefinitionSource::Api)
        .await
        .expect("v2 publish");
    assert!(
        matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Structural,
                ..
            }
        ),
        "v2 must be classified Structural, got {out:?}"
    );

    // The active topology pointer MUST have moved.
    let v2_topology = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("still mounted")
        .load();
    assert!(
        !Arc::ptr_eq(&v1_topology, &v2_topology),
        "structural publish must swap the active topology pointer"
    );

    // R1's cancel handle MUST NOT have fired (drain policy).
    assert!(
        !r1.cancel.is_cancelled(),
        "drain policy must not cancel in-flight runs on structural swap"
    );

    // ---------- release R1; observe it complete against the OLD snapshot ----------
    gate.notify_waiters();
    let status = timeout(Duration::from_secs(2), r1.join)
        .await
        .expect("R1 did not complete after gate release")
        .expect("R1 coordinator panicked");
    assert_eq!(status, RunStatus::Completed);
    assert_eq!(
        log_calls.load(Ordering::SeqCst),
        1,
        "R1's log must fire exactly once"
    );
    assert_eq!(
        extra_calls.load(Ordering::SeqCst),
        0,
        "R1 must NOT fire `extra` (it lives in v2, not R1's v1 snapshot)"
    );

    // ---------- R2 against the v2 snapshot: extra DOES fire ----------
    let r2_topology = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("still mounted")
        .load();

    let spec2 = RunSpec::new(
        flow.clone(),
        FlowRevisionId::new(),
        Arc::clone(&r2_topology),
        // Use a DIFFERENT seed value so R3 idempotent-write
        // short-circuit doesn't drop the slot write (R1 left
        // agent.in = Int(1) in the GraphStore).
        vec![(slot("smoke.agent", "in"), SlotValue::Int(2))],
        vec![slot("smoke.extra", "out")],
    );
    let r2 = runner.start(spec2, SlotMap::new()).await.expect("R2 start");

    // R2's agent will park on the gate; give it a moment, then open.
    for _ in 0..50 {
        if agent_calls.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    gate.notify_waiters();

    let status = timeout(Duration::from_secs(2), r2.join)
        .await
        .expect("R2 did not complete")
        .expect("R2 coordinator panicked");
    assert_eq!(status, RunStatus::Completed);
    assert_eq!(
        log_calls.load(Ordering::SeqCst),
        2,
        "R2's log must fire (now 2 cumulative)"
    );
    assert_eq!(
        extra_calls.load(Ordering::SeqCst),
        1,
        "R2 MUST fire `extra` (v2 snapshot)"
    );

    // Definition bus: at least one SwapApplied between R1 and now.
    let mut saw_swap = false;
    for _ in 0..8 {
        match timeout(Duration::from_millis(80), def_rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::SwapApplied { .. })) => {
                saw_swap = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_swap, "structural publish must emit SwapApplied");
}
