//! Integration test for the `starter.flow.trigger.schedule` body.
//!
//! Phase A.3 of the `rubix-goal-6-weekly-report` job. Drives a tiny
//! one-node flow rooted at `trigger.schedule` through the real
//! `propagator` + `GraphStore` and asserts the configured `cron_expr`
//! surfaces on the node's `schedule` output slot **unmodified** — the
//! property `FlowAsService` relies on to enumerate a flow's schedules
//! without re-parsing the YAML body.
//!
//! `trigger.schedule` is a passive entry node — the actual firing
//! comes from the host-side durable cron scheduler (Phase B). This
//! test only exercises the body's pass-through contract.

#![cfg(feature = "trigger-schedule")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::{self, FlowTopology, PropagatorConfig};
use starter_flow::run::RunCancel;
use starter_flow_nodes::node_registry::StaticNodeKindRegistry;
use starter_flow_nodes::trigger_schedule::{
    self, TriggerSchedule, CRON_EXPR_SLOT, KIND_ID, SCHEDULE_SLOT,
};
use starter_flow_spi::flow::{FlowEvent, RunId};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, NodeKindRegistry, SlotRef, SlotValue};
use starter_flow_spi::Cancel;
use tokio::sync::broadcast;
use tokio::time::timeout;

fn nid(s: &str) -> NodeId {
    NodeId::new(s).unwrap()
}
fn slot(node: &str, name: &str) -> SlotRef {
    SlotRef::new(nid(node), name)
}

fn one_node_topology() -> Arc<FlowTopology> {
    let node = nid("flow.test.ts");
    let body: Arc<dyn NodeBehavior> = Arc::new(TriggerSchedule::new());

    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), BTreeSet::from([CRON_EXPR_SLOT.to_owned()]));

    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(node, body);

    Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        reads: BTreeMap::new(),
        behaviors,
    })
}

/// The kind id registers in a [`StaticNodeKindRegistry`] under the
/// reserved `starter.flow.trigger.schedule` reverse-DNS namespace —
/// the catalog surface FlowAsService consults to discover scheduled
/// flows.
#[test]
fn kind_registers_in_static_registry() {
    let reg = StaticNodeKindRegistry::with_builtins();
    let kind = KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id");
    let descriptor = reg
        .lookup(&kind)
        .expect("trigger.schedule descriptor must be registered with builtins");
    assert_eq!(descriptor.kind.as_ref(), KIND_ID);
    // Sanity-check the catalog keys match the module's static descriptor.
    assert_eq!(descriptor.kind, trigger_schedule::DESCRIPTOR.kind);
}

/// Drive a one-node flow rooted at `trigger.schedule` with
/// `cron_expr = "0 0 * * 0"` (weekly Sunday midnight) through the
/// real propagator and assert the `schedule` output slot exposes the
/// expression unmodified.
#[tokio::test]
async fn weekly_cron_expr_passes_through_schedule_slot() {
    const CRON: &str = "0 0 * * 0";

    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let topology = one_node_topology();
    let cancel_arc = RunCancel::new();
    let cancel: Arc<dyn Cancel> = cancel_arc.clone();
    let (events_tx, mut events_rx) = broadcast::channel::<FlowEvent>(64);

    let handle = propagator::spawn(
        store.clone(),
        topology,
        cancel,
        events_tx.clone(),
        RunId::new(),
        PropagatorConfig::default(),
    );

    // Write the cron_expr config slot — that's the trigger.
    store
        .write_slot(
            &slot("flow.test.ts", CRON_EXPR_SLOT),
            SlotValue::String(CRON.to_owned()),
            WriteSlotOpts::live(),
        )
        .await
        .expect("seed cron_expr slot");

    // Wait for the `NodeEmitted { slot: "schedule", .. }` event to
    // confirm the body ran and emitted into the propagator's chokepoint.
    let deadline = Duration::from_millis(500);
    let mut saw_schedule_emit = false;
    let mut emitted_value: Option<SlotValue> = None;

    let collect = async {
        loop {
            match events_rx.recv().await {
                Ok(FlowEvent::NodeEmitted { node, slot, value, .. })
                    if node.as_str() == "flow.test.ts" && slot == SCHEDULE_SLOT =>
                {
                    saw_schedule_emit = true;
                    emitted_value = Some(value);
                    break;
                }
                Ok(FlowEvent::NodeFailed { node, .. })
                    if node.as_str() == "flow.test.ts" =>
                {
                    panic!("trigger.schedule body must not fail on a valid cron_expr");
                }
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    let _ = timeout(deadline, collect).await;

    assert!(
        saw_schedule_emit,
        "expected a NodeEmitted event on the `schedule` output slot",
    );
    match emitted_value {
        Some(SlotValue::String(s)) => assert_eq!(
            s, CRON,
            "trigger.schedule must expose the cron expression unmodified",
        ),
        other => panic!("expected SlotValue::String on `schedule`; got {other:?}"),
    }

    // Also read straight from the store — the propagator's R2 write
    // chokepoint must have funnelled the output through write_slot.
    let stored = store
        .read_slot(&slot("flow.test.ts", SCHEDULE_SLOT))
        .await
        .expect("schedule slot must be present in the store");
    match stored {
        SlotValue::String(s) => assert_eq!(s, CRON),
        other => panic!("expected SlotValue::String in store; got {other:?}"),
    }

    cancel_arc.cancel();
    let _ = timeout(Duration::from_secs(1), handle).await;
}
