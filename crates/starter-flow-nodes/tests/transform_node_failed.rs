//! End-to-end integration test for the `starter.flow.transform` body.
//!
//! Phase 2 stage 3 of the `starter-flow-engine-finish` job. The unit
//! tests inside `crates/starter-flow-nodes/src/transform.rs` cover the
//! direct `NodeBehavior::invoke` contract — identity / sum / panic /
//! unregistered. This file drives a panicking transform AND an
//! unregistered transform through the real propagator + `GraphStore`
//! and asserts the failure surfaces as [`FlowEvent::NodeFailed`] on
//! the run's event stream — *not* as a propagator-task crash. That's
//! the "panic surfaces as NodeFailed" property the stage-3 brief
//! calls out.
//!
//! We drive [`starter_flow::propagator::spawn`] directly rather than
//! [`starter_flow::run::FlowRunner`] because `RunSpec` is
//! `#[non_exhaustive]` and stage 3 is not a place to add engine
//! constructors. The propagator is the seam that actually maps
//! `Err(NodeError)` to `FlowEvent::NodeFailed`, so testing it directly
//! is the closest proof of the property we want.

#![cfg(feature = "transform")]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::{self, FlowTopology, PropagatorConfig};
use starter_flow::run::RunCancel;
use starter_flow_nodes::transform::{
    StaticTransformFunctionRegistry, Transform, TransformFunctionRegistry, FN_ID_SLOT,
};
use starter_flow_spi::flow::{FlowEvent, RunId};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Cancel;
use tokio::sync::broadcast;
use tokio::time::timeout;

fn nid(s: &str) -> NodeId {
    NodeId::new(s).unwrap()
}
fn slot(node: &str, name: &str) -> SlotRef {
    SlotRef::new(nid(node), name)
}

fn one_node_transform_topology(registry: Arc<dyn TransformFunctionRegistry>) -> Arc<FlowTopology> {
    let node = nid("flow.test.t");
    let transform: Arc<dyn NodeBehavior> = Arc::new(Transform::new(registry));

    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(
        node.clone(),
        BTreeSet::from([FN_ID_SLOT.to_owned(), "payload".to_owned()]),
    );

    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(node, transform);

    Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        reads: BTreeMap::new(),
        behaviors,
    })
}

async fn drain(rx: &mut broadcast::Receiver<FlowEvent>, dur: Duration) -> Vec<FlowEvent> {
    let mut out = Vec::new();
    loop {
        match timeout(dur, rx.recv()).await {
            Ok(Ok(ev)) => out.push(ev),
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(broadcast::error::RecvError::Closed)) => break,
            Err(_) => break,
        }
    }
    out
}

async fn drive_and_collect(
    registry: Arc<dyn TransformFunctionRegistry>,
    fn_id: &str,
) -> Vec<FlowEvent> {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let topology = one_node_transform_topology(registry);
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

    // Pre-seed fn_id, then write `payload` to kick the propagator —
    // either write triggers, but funnelling both means the propagator
    // reads the full input map (R2 chokepoint, single path).
    store
        .write_slot(
            &slot("flow.test.t", FN_ID_SLOT),
            SlotValue::String(fn_id.to_owned()),
            WriteSlotOpts::live(),
        )
        .await
        .unwrap();
    store
        .write_slot(
            &slot("flow.test.t", "payload"),
            SlotValue::String("anything".to_owned()),
            WriteSlotOpts::live(),
        )
        .await
        .unwrap();

    let events = drain(&mut events_rx, Duration::from_millis(200)).await;

    cancel_arc.cancel();
    let _ = timeout(Duration::from_secs(1), handle).await;
    events
}

/// A panicking transform surfaces as `FlowEvent::NodeFailed`, not as
/// a propagator-task crash.
#[tokio::test]
async fn panicking_transform_surfaces_as_node_failed() {
    let mut registry = StaticTransformFunctionRegistry::new();
    registry.register("boom", |_input: SlotMap| -> SlotMap {
        panic!("intentional propagator-level test panic")
    });
    let registry: Arc<dyn TransformFunctionRegistry> = Arc::new(registry);

    let events = drive_and_collect(registry, "boom").await;
    let found = events
        .iter()
        .any(|e| matches!(e, FlowEvent::NodeFailed { node, .. } if node.as_str() == "flow.test.t"));
    assert!(
        found,
        "panicking transform must surface as FlowEvent::NodeFailed on flow.test.t; got {events:?}",
    );
}

/// A transform whose `fn_id` is unregistered also surfaces as
/// `FlowEvent::NodeFailed`.
#[tokio::test]
async fn unregistered_fn_id_surfaces_as_node_failed() {
    let registry: Arc<dyn TransformFunctionRegistry> =
        Arc::new(StaticTransformFunctionRegistry::new());

    let events = drive_and_collect(registry, "does-not-exist").await;
    let found = events
        .iter()
        .any(|e| matches!(e, FlowEvent::NodeFailed { node, .. } if node.as_str() == "flow.test.t"));
    assert!(
        found,
        "unregistered fn_id must surface as FlowEvent::NodeFailed on flow.test.t; got {events:?}",
    );
}
