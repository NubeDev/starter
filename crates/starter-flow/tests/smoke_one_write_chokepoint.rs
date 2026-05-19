//! SCOPE Smoke test: **One write chokepoint** (R2).
//!
//! Three writers all enter [`GraphStore::write_slot`] for the same
//! `(NodeId, slot)` pair:
//!
//! 1. A REST-style direct call dispatched from a thin `tokio::spawn`
//!    task (the surface adapter shape).
//! 2. A CLI-style synchronous call from the test thread itself (the
//!    shape `starter-cli` produces today).
//! 3. An internal propagator tick: the test seeds a transform node's
//!    input slot through the same chokepoint, the propagator triggers
//!    the node, and the node's behaviour writes back into the target
//!    slot through `GraphStore::write_slot`.
//!
//! Two assertions cover the single-chokepoint invariant:
//!
//! - A `tracing-subscriber` `Layer` counts every `write_slot` span
//!   whose `node_id` + `slot_name` fields name the target slot and
//!   asserts the count is **exactly three**. Any bypass — a future
//!   refactor that performs a slot mutation without going through
//!   `GraphStore::write_slot` — drops the count below three and fails
//!   the test.
//! - A live `GraphStore::subscribe()` receiver counts `SlotChanged`
//!   envelopes targeting the same slot and asserts the count is
//!   **exactly three** (three distinct values defeat the R3
//!   idempotent-write short-circuit so all three propagate).
//!
//! If a hardcoded path bypasses the chokepoint, the span counter
//! drops; if R3 idempotency swallows a write, the envelope counter
//! drops. Either fails this test loudly.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::time::timeout;

use async_trait::async_trait;
use starter_flow_spi::flow::RunId;
use starter_flow_spi::graph::{GraphStore, SubscribeOpts, WriteSlotOpts};
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::{self, FlowTopology, PropagatorConfig};
use starter_flow::run::RunCancel;

use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::Subscriber;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

// ---------------------------------------------------------------------------
// tracing-subscriber Layer: count write_slot spans on the target slot.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FieldGrabber {
    node: Option<String>,
    slot: Option<String>,
}

impl Visit for FieldGrabber {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "node_id" => self.node = Some(value.to_owned()),
            "slot_name" => self.slot = Some(value.to_owned()),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        // `%expr` records via `record_debug` with the Display
        // formatting baked in; strip surrounding quotes that
        // `Debug::fmt` adds to string values.
        let raw = format!("{value:?}");
        let trimmed = raw.trim_matches('"').to_owned();
        match field.name() {
            "node_id" => self.node = Some(trimmed),
            "slot_name" => self.slot = Some(trimmed),
            _ => {}
        }
    }
}

struct WriteSlotCounter {
    target_node: String,
    target_slot: String,
    count: Arc<Mutex<usize>>,
}

impl<S> Layer<S> for WriteSlotCounter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &tracing::span::Id, _ctx: Context<'_, S>) {
        if attrs.metadata().name() != "write_slot" {
            return;
        }
        let mut g = FieldGrabber::default();
        attrs.record(&mut g);
        if g.node.as_deref() == Some(self.target_node.as_str())
            && g.slot.as_deref() == Some(self.target_slot.as_str())
        {
            *self.count.lock().unwrap() += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Transform-style NodeBehavior: writes input value back to the same
// node's output slot. The propagator's `write_slot` call into that
// output slot is the third write the smoke test counts.
// ---------------------------------------------------------------------------

struct EchoBehavior {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for EchoBehavior {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let v = input.get("in").cloned().unwrap_or(SlotValue::Null);
        let mut out = SlotMap::new();
        out.insert("value".to_owned(), v);
        Ok(out)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_writers_share_one_chokepoint() {
    // Target slot every writer targets.
    let target_node = NodeId::new("com.acme.target").unwrap();
    let target = SlotRef::new(target_node.clone(), "value");

    // ---- tracing subscriber: count `write_slot` spans on the target. ----
    let count = Arc::new(Mutex::new(0usize));
    let layer = WriteSlotCounter {
        target_node: target_node.as_str().to_owned(),
        target_slot: "value".to_owned(),
        count: count.clone(),
    };
    // `try_init` so re-running the test in the same binary (cargo
    // test harness may reuse a process) does not panic.
    let _ = tracing_subscriber::registry().with(layer).try_init();

    // ---- store + propagator wiring. ----
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

    // Topology: the target node `com.acme.target` is a transform that
    // triggers on its `in` slot and produces a `value` output. The
    // propagator's write to (com.acme.target, value) is writer #3.
    let mut triggers = std::collections::BTreeMap::new();
    triggers.insert(
        target_node.clone(),
        std::iter::once("in".to_owned()).collect(),
    );
    let mut behaviors = std::collections::BTreeMap::new();
    behaviors.insert(
        target_node.clone(),
        Arc::new(EchoBehavior {
            kind: KindId::new("starter.flow.test-echo").unwrap(),
        }) as Arc<dyn NodeBehavior>,
    );
    let topology = Arc::new(FlowTopology {
        links: Default::default(),
        triggers,
        behaviors,
    });

    let cancel = RunCancel::new();
    let (events_tx, _events_rx) = broadcast::channel(64);
    let _prop = propagator::spawn(
        store.clone(),
        topology,
        cancel.clone(),
        events_tx,
        RunId::new(),
        PropagatorConfig::default(),
    );

    // ---- store subscription: count SlotChanged envelopes on target.
    //
    // Subscribe *after* the propagator (the propagator subscribed
    // synchronously inside `spawn`) but *before* any write, so we see
    // every event the writers produce.
    let mut sub = store.subscribe(SubscribeOpts::default());

    // ---- Writer #1: REST-style direct call from a tokio task. ----
    {
        let store = store.clone();
        let target = target.clone();
        tokio::spawn(async move {
            store
                .write_slot(&target, SlotValue::Int(1), WriteSlotOpts::live())
                .await
                .unwrap();
        })
        .await
        .unwrap();
    }

    // ---- Writer #2: CLI-style synchronous call from the test thread. ----
    store
        .write_slot(&target, SlotValue::Int(2), WriteSlotOpts::live())
        .await
        .unwrap();

    // ---- Writer #3: seed the transform's input so the propagator
    //                  triggers the node and writes the target. ----
    let seed = SlotRef::new(target_node.clone(), "in");
    store
        .write_slot(&seed, SlotValue::Int(3), WriteSlotOpts::live())
        .await
        .unwrap();

    // Give the propagator a beat to consume the seed event, invoke
    // the echo behaviour, and write the target's `value` slot.
    // (Quiescence is well below the 200 ms unit-test budget.)
    tokio::time::sleep(Duration::from_millis(200)).await;

    // ---- Assertion 1: exactly three spans on the chokepoint. ----
    let span_count = *count.lock().unwrap();
    assert_eq!(
        span_count, 3,
        "expected exactly three `write_slot` spans on the target slot, observed {span_count}",
    );

    // ---- Assertion 2: exactly three SlotChanged envelopes on target.
    let mut envelope_count = 0usize;
    let mut observed_values: Vec<SlotValue> = Vec::new();
    while let Ok(Some(env)) = timeout(Duration::from_millis(50), sub.next()).await {
        if env.slot != target {
            continue;
        }
        if let Some(v) = env.value {
            observed_values.push(v);
        }
        envelope_count += 1;
        if envelope_count >= 3 {
            // Drain a moment more to confirm no extras follow.
            break;
        }
    }
    // Drain any tail.
    while let Ok(Some(_)) = timeout(Duration::from_millis(20), sub.next()).await {}

    assert_eq!(
        envelope_count, 3,
        "expected exactly three SlotChanged envelopes on the target slot, observed {envelope_count} (values: {observed_values:?})",
    );

    // Three distinct values defeat the R3 idempotent-write short-
    // circuit; every value the writers produced must show up.
    assert!(observed_values.contains(&SlotValue::Int(1)));
    assert!(observed_values.contains(&SlotValue::Int(2)));
    assert!(observed_values.contains(&SlotValue::Int(3)));

    cancel.cancel();
}
