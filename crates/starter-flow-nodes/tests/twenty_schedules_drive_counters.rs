//! End-to-end engine smoke test: 20 schedule trigger nodes, each
//! linked to its own counter, all firing N times. Verifies that
//! every counter ends at exactly N — no double-fires, no dropped
//! invocations, no cross-talk between the 20 sub-pipelines.
//!
//! This is the simplest "does the engine work at all" question
//! the data-flow stage needs answered before any surface wiring
//! gets blamed. Nothing in here touches rubix surfaces, the
//! durable scheduler, MCP, Postgres, or ClickHouse — it drives
//! the propagator directly.

#![cfg(all(feature = "trigger-schedule", feature = "counter"))]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::timeout;

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::{self, FlowTopology, PropagatorConfig};
use starter_flow::run::RunCancel;
use starter_flow::state::in_memory::InMemoryNodeStateStore;
use starter_flow_nodes::counter::{Counter, CounterSettings, IN_SLOT as COUNTER_IN, OUT_SLOT};
use starter_flow_nodes::trigger_schedule::{TriggerSchedule, CRON_EXPR_SLOT, FIRE_SLOT};
use starter_flow_spi::flow::{FlowId, RunId};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{NodeBehavior, NodeId, SlotRef, SlotValue};

const N_PIPELINES: usize = 20;
const N_FIRES: usize = 50;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_schedules_each_drive_their_counter_exactly_n_times() {
    // ----- topology: 20 (schedule -> counter) pairs, no cross-links.
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();

    let mut schedule_ids: Vec<NodeId> = Vec::with_capacity(N_PIPELINES);
    let mut counter_ids: Vec<NodeId> = Vec::with_capacity(N_PIPELINES);

    for i in 0..N_PIPELINES {
        let sched = NodeId::new(&format!("com.acme.sched.s{i:02}")).unwrap();
        let cnt = NodeId::new(&format!("com.acme.cnt.c{i:02}")).unwrap();

        // schedule fires when its `cron_expr` slot is written.
        triggers.insert(
            sched.clone(),
            std::iter::once(CRON_EXPR_SLOT.to_owned()).collect(),
        );
        // counter fires when its `in` slot is written.
        triggers.insert(
            cnt.clone(),
            std::iter::once(COUNTER_IN.to_owned()).collect(),
        );

        behaviors.insert(
            sched.clone(),
            Arc::new(TriggerSchedule::new()) as Arc<dyn NodeBehavior>,
        );
        behaviors.insert(
            cnt.clone(),
            Arc::new(Counter::with_settings(CounterSettings::default())) as Arc<dyn NodeBehavior>,
        );

        // link: sched.fire -> cnt.in
        links
            .entry(SlotRef::new(sched.clone(), FIRE_SLOT))
            .or_default()
            .push(SlotRef::new(cnt.clone(), COUNTER_IN));

        schedule_ids.push(sched);
        counter_ids.push(cnt);
    }

    let topology = Arc::new(FlowTopology {
        links,
        triggers,
        reads: BTreeMap::new(),
        behaviors,
    });

    // ----- store + state + propagator.
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::with_capacity(16_384));
    let node_state: Arc<dyn starter_flow_spi::state::NodeStateStore> =
        Arc::new(InMemoryNodeStateStore::new());

    let cancel = RunCancel::new();
    let (events_tx, _events_rx) = broadcast::channel(1024);
    let flow_id = FlowId::new("com.acme.twenty-schedules.test").unwrap();
    let _prop = propagator::spawn_with_checkpoint(
        store.clone(),
        topology,
        cancel.clone(),
        events_tx,
        RunId::new(),
        {
            let mut cfg = PropagatorConfig::default();
            cfg.max_propagation_hops = 1_000_000;
            cfg
        },
        None,
        Arc::new(starter_flow_spi::skill::SkillSelection::None),
        node_state.clone(),
        Some(flow_id),
    );

    // ----- drive N_FIRES fires across all 20 pipelines.
    //
    // Each "fire" writes a fresh cron_expr value to every schedule
    // node. We use a per-fire suffix so the R3 idempotent-write
    // short-circuit doesn't swallow re-writes of the same string —
    // the schedule body ignores the cron's *value*, it just emits
    // `fire_ms` on every invocation.
    for n in 0..N_FIRES {
        for sched in &schedule_ids {
            let slot = SlotRef::new(sched.clone(), CRON_EXPR_SLOT);
            let cron = format!("*/{} * * * * *", n + 1); // each fire uses a distinct value
            store
                .write_slot(&slot, SlotValue::String(cron), WriteSlotOpts::live())
                .await
                .expect("seed cron_expr");
        }
        // Give the propagator a beat between fires so the chain
        // (sched -> cnt) drains before the next fire arrives.
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // ----- wait for full drain.
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // ----- read every counter's `out` slot and assert == N_FIRES.
    let mut bad: Vec<(String, i64)> = Vec::new();
    for cnt in &counter_ids {
        let slot = SlotRef::new(cnt.clone(), OUT_SLOT);
        let v = timeout(Duration::from_millis(200), store.read_slot(&slot))
            .await
            .expect("read counter timed out")
            .unwrap_or_else(|e| panic!("read {cnt}.{OUT_SLOT}: {e}"));
        match v {
            SlotValue::Int(n) if n as usize == N_FIRES => {}
            other => bad.push((cnt.as_str().to_owned(), int_or_neg1(&other))),
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} counters didn't reach {}: {:?}",
        bad.len(),
        N_PIPELINES,
        N_FIRES,
        bad,
    );

    cancel.cancel();
}

fn int_or_neg1(v: &SlotValue) -> i64 {
    match v {
        SlotValue::Int(n) => *n,
        _ => -1,
    }
}
