//! Regression: writing to a node's *read* slot must not wake the
//! node. Triggers are the subscribe set; reads are configuration /
//! reference inputs assembled into the input `SlotMap` at invoke
//! time but never on their own a wake signal.
//!
//! This is the engine-level proof for the fix described in
//! `rubix/docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire-root-cause.md`:
//! a per-fire seed write of a config-style slot (e.g. the tool-call
//! kind's `tool_id`) used to wake the destination node spuriously
//! because "triggers" did double duty as both the wake set and the
//! input-read set. After the SPI split the wake set is exactly
//! `NodeBehavior::trigger_slots`; the input read set is
//! `trigger_slots ∪ read_slots`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::{self, FlowTopology, PropagatorConfig};
use starter_flow::run::RunCancel;
use starter_flow::state::in_memory::InMemoryNodeStateStore;
use starter_flow_spi::flow::{FlowEvent, FlowId, RunId};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};

/// Behaviour that records every invocation. Declares `wake` as a
/// trigger slot and `cfg` as a read slot.
struct ReadVsTriggerProbe {
    kind: KindId,
    invokes: Arc<AtomicUsize>,
    last_cfg: Arc<std::sync::Mutex<Option<SlotValue>>>,
}

impl ReadVsTriggerProbe {
    fn new() -> (
        Self,
        Arc<AtomicUsize>,
        Arc<std::sync::Mutex<Option<SlotValue>>>,
    ) {
        let invokes = Arc::new(AtomicUsize::new(0));
        let last = Arc::new(std::sync::Mutex::new(None));
        let me = Self {
            kind: KindId::new("test.read-vs-trigger.probe").unwrap(),
            invokes: invokes.clone(),
            last_cfg: last.clone(),
        };
        (me, invokes, last)
    }
}

#[async_trait]
impl NodeBehavior for ReadVsTriggerProbe {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    fn trigger_slots(&self) -> &'static [&'static str] {
        &["wake"]
    }

    fn read_slots(&self) -> &'static [&'static str] {
        &["cfg"]
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        self.invokes.fetch_add(1, Ordering::SeqCst);
        let mut guard = self.last_cfg.lock().unwrap();
        *guard = input.get("cfg").cloned();
        Ok(SlotMap::new())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writes_to_read_slot_do_not_wake_but_are_visible_on_next_wake() {
    let (probe, invokes, last_cfg) = ReadVsTriggerProbe::new();
    let node = NodeId::new("com.test.probe").unwrap();

    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("wake".to_owned()).collect());
    let mut reads: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    reads.insert(node.clone(), std::iter::once("cfg".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(node.clone(), Arc::new(probe) as Arc<dyn NodeBehavior>);

    let topology = Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        reads,
        behaviors,
    });

    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::with_capacity(1024));
    let node_state: Arc<dyn starter_flow_spi::state::NodeStateStore> =
        Arc::new(InMemoryNodeStateStore::new());

    let cancel = RunCancel::new();
    let (events_tx, _events_rx) = broadcast::channel::<FlowEvent>(64);
    let flow_id = FlowId::new("com.test.read-vs-trigger").unwrap();
    let _prop = propagator::spawn_with_checkpoint(
        store.clone(),
        topology,
        cancel.clone(),
        events_tx,
        RunId::new(),
        PropagatorConfig::default(),
        None,
        Arc::new(starter_flow_spi::skill::SkillSelection::None),
        node_state,
        Some(flow_id),
    );

    // --- 1) write to `cfg` repeatedly: read-only writes, must NOT wake.
    for i in 0..10 {
        store
            .write_slot(
                &SlotRef::new(node.clone(), "cfg".to_owned()),
                SlotValue::Int(i),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();
    }
    // Give the propagator a chance to (incorrectly) wake.
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        invokes.load(Ordering::SeqCst),
        0,
        "writes to a read slot must not invoke the node"
    );

    // --- 2) write to `wake`: triggers exactly one invocation, and
    // the input map carries the latest `cfg` value (9).
    store
        .write_slot(
            &SlotRef::new(node.clone(), "wake".to_owned()),
            SlotValue::String("kick-1".to_owned()),
            WriteSlotOpts::live(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        invokes.load(Ordering::SeqCst),
        1,
        "one write to the trigger slot must produce exactly one invoke",
    );
    let seen = last_cfg.lock().unwrap().clone();
    assert_eq!(
        seen,
        Some(SlotValue::Int(9)),
        "read slot's latest value must be visible in invoke's input map",
    );

    // --- 3) another batch of read-only writes after a wake: still
    // no spurious wake.
    for i in 100..120 {
        store
            .write_slot(
                &SlotRef::new(node.clone(), "cfg".to_owned()),
                SlotValue::Int(i),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        invokes.load(Ordering::SeqCst),
        1,
        "second batch of read-slot writes must not wake",
    );

    // --- 4) trigger again. Invoke count climbs by exactly one and
    // sees the new `cfg` value (119).
    store
        .write_slot(
            &SlotRef::new(node.clone(), "wake".to_owned()),
            SlotValue::String("kick-2".to_owned()),
            WriteSlotOpts::live(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(invokes.load(Ordering::SeqCst), 2);
    let seen = last_cfg.lock().unwrap().clone();
    assert_eq!(seen, Some(SlotValue::Int(119)));

    cancel.cancel();
}
