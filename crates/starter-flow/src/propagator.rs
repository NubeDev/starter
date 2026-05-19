//! Synchronous tokio propagator loop per R2 and the rubix
//! `live_wire.rs` Decisions reference.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" — the single
//! synchronous loop that subscribes to [`GraphStore`] slot changes,
//! copies values along outbound links to downstream input slots,
//! invokes the destination [`NodeBehavior`] when a triggering input
//! slot changes, and writes the resulting outputs back through the
//! one [`GraphStore::write_slot`] chokepoint (R2).
//!
//! ## Invariants this stage owns
//!
//! - **One chokepoint, no slot ownership.** The propagator never holds
//!   slot data; every value it forwards goes through
//!   [`GraphStore::write_slot`] (R2 — "the propagator is a reader,
//!   not an owner").
//! - **Per-run propagation budget.** The propagator owns a hop counter
//!   incremented on every event it consumes; when the counter exceeds
//!   [`PropagatorConfig::max_propagation_hops`] (default
//!   [`DEFAULT_MAX_PROPAGATION_HOPS`] = 1000, per Phase 2 stage 1
//!   "Decisions made"), the run is failed with
//!   [`FlowError::CycleBudgetExhausted`] and a
//!   [`FlowEvent::RunFailed`] is emitted before the loop exits.
//! - **Idempotent-write short-circuit hooked from stage 3.** Because
//!   every downstream propagation goes through [`GraphStore::write_slot`],
//!   the R3 short-circuit implemented by [`InMemoryGraphStore`] (no
//!   `SlotChanged` if new value equals prior value) naturally stops
//!   trivial cycles without the propagator needing to know about it
//!   — it just never sees the next event.
//! - **Cancel within a few hundred ms.** The main loop awaits
//!   [`Cancel::cancelled`] in a [`tokio::select!`] alongside the
//!   subscription stream, so a fired cancel stops scheduling further
//!   hops on the next yield point — well within the SCOPE budget. A
//!   borrow of the [`Cancel`] handle is also threaded through
//!   [`NodeCtx`] into every [`NodeBehavior::invoke`] call so node
//!   bodies can abort their own work.
//!
//! [`InMemoryGraphStore`]: crate::graph::InMemoryGraphStore

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use starter_flow_spi::flow::{FlowError, FlowEvent, RunId};
use starter_flow_spi::graph::{GraphStore, SubscribeOpts, SubscriptionStream, WriteSlotOpts};
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotRef};
use starter_flow_spi::Cancel;

/// Default per-run propagation budget — locked at Phase 2 stage 1
/// "Decisions made" (R1 cycle-bound budget defaults).
pub const DEFAULT_MAX_PROPAGATION_HOPS: u64 = 1000;

/// Propagator policies exposed at `FlowRunner::start` time.
///
/// Both knobs land here in stage 4:
/// - `max_propagation_hops` — the R1 cycle-bound budget (default 1000,
///   overridable per `FlowRunner::start` call).
/// - The R3 idempotent-write short-circuit is hooked from stage 3 via
///   [`GraphStore::write_slot`] itself; it has no extra knob on the
///   propagator because the short-circuit is store-level (the
///   propagator simply consumes whichever `SlotChanged` events the
///   store chooses to emit).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct PropagatorConfig {
    /// Maximum number of `SlotChanged` events the propagator will
    /// consume in a single run before failing it with
    /// [`FlowError::CycleBudgetExhausted`].
    pub max_propagation_hops: u64,
}

impl Default for PropagatorConfig {
    fn default() -> Self {
        Self {
            max_propagation_hops: DEFAULT_MAX_PROPAGATION_HOPS,
        }
    }
}

/// The engine-internal flow topology the propagator drives.
///
/// Phase 2 keeps this dead simple — it is the minimum shape the
/// propagator needs to do its job, expressed as plain maps. The
/// `FlowRegistry` and `NodeKindRegistry` (which compute this from a
/// persisted flow definition) land in Phase 3; for stage 4 the
/// engine constructs a [`FlowTopology`] directly from in-test fixtures
/// or, eventually, from the registry output.
///
/// Field semantics:
///
/// - [`Self::links`] — outbound slot-to-slot links. When the source
///   [`SlotRef`] changes, the propagator copies the value into each
///   destination [`SlotRef`] via [`GraphStore::write_slot`]. This is
///   the rubix `live_wire.rs` model (R2 + "live wires copy values
///   along edges").
/// - [`Self::triggers`] — for each node, the set of input slot *names*
///   whose change triggers an invocation of the node. When the
///   propagator sees a `SlotChanged` whose `(node, slot)` matches
///   `triggers[node]`, it gathers the node's input map and calls
///   `NodeBehavior::invoke`.
/// - [`Self::behaviors`] — the [`NodeBehavior`] impl for each node id.
///   In Phase 3 this is computed by `NodeKindRegistry::resolve`; in
///   Phase 2 stage 4 it is supplied directly by the engine harness
///   that wires the propagator up.
#[derive(Default, Clone)]
pub struct FlowTopology {
    /// Outbound link map: src slot → list of downstream input slots.
    ///
    /// Keyed by [`SlotRef`] (which is `Hash + Eq` but not `Ord`).
    pub links: HashMap<SlotRef, Vec<SlotRef>>,
    /// Per-node trigger-input slot names.
    pub triggers: BTreeMap<NodeId, BTreeSet<String>>,
    /// Per-node behavior implementation.
    pub behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>>,
}

/// Spawn the propagator task driving one run.
///
/// Returns the [`JoinHandle`] for the spawned task. The task exits
/// when:
///
/// - the [`Cancel`] handle fires (emits [`FlowEvent::RunCancelled`]),
/// - the propagation budget is exhausted (emits [`FlowEvent::RunFailed`]
///   with [`FlowError::CycleBudgetExhausted`]),
/// - or the [`GraphStore`] subscription stream closes (clean exit, no
///   event emitted).
///
/// The propagator never closes the `events` broadcast channel itself —
/// the engine that holds the `Sender` decides when to drop it.
pub fn spawn(
    store: Arc<dyn GraphStore>,
    topology: Arc<FlowTopology>,
    cancel: Arc<dyn Cancel>,
    events: broadcast::Sender<FlowEvent>,
    run: RunId,
    config: PropagatorConfig,
) -> JoinHandle<()> {
    // Subscribe *synchronously* before spawning the task so that any
    // writes the caller performs immediately after `spawn()` returns
    // are guaranteed to land in the propagator's subscription queue
    // — no "did the spawned task get to `subscribe` first?" races.
    let sub = store.subscribe(SubscribeOpts::default());
    tokio::spawn(drive(store, sub, topology, cancel, events, run, config))
}

/// Body of the propagator task. Public for ergonomics ([`spawn`] is
/// the recommended entry point) but kept on a stable surface so smoke
/// tests and Phase 3 engine wiring can drive the loop inline.
pub async fn drive(
    store: Arc<dyn GraphStore>,
    mut sub: SubscriptionStream,
    topology: Arc<FlowTopology>,
    cancel: Arc<dyn Cancel>,
    events: broadcast::Sender<FlowEvent>,
    run: RunId,
    config: PropagatorConfig,
) {
    let mut hops: u64 = 0;

    loop {
        tokio::select! {
            biased;
            // R13: fired Cancel stops the propagator scheduling more
            // hops within a few hundred ms. `biased` puts Cancel
            // ahead of the subscription poll so a cancel that arrives
            // mid-step short-circuits the next iteration.
            _ = cancel.cancelled() => {
                let _ = events.send(FlowEvent::RunCancelled { run });
                return;
            }
            next = sub.next() => {
                let Some(env) = next else {
                    // Subscription stream closed — clean exit.
                    return;
                };
                let Some(value) = env.value.clone() else {
                    continue;
                };

                hops = hops.saturating_add(1);
                if hops > config.max_propagation_hops {
                    let err = FlowError::CycleBudgetExhausted { hops };
                    tracing::warn!(
                        run = %run,
                        hops,
                        max = config.max_propagation_hops,
                        "propagator cycle budget exhausted",
                    );
                    let _ = events.send(FlowEvent::run_failed(run, &err));
                    return;
                }

                // 1. Fan out along outbound links — copy the value into
                //    each downstream input slot through the single
                //    write_slot chokepoint. The store's R3
                //    idempotent-write short-circuit (stage 3) handles
                //    trivial cycle termination for us: if the
                //    downstream slot already holds this value, no
                //    further `SlotChanged` event is emitted and the
                //    chain dies on its own.
                if let Some(dsts) = topology.links.get(&env.slot) {
                    for dst in dsts {
                        if cancel.is_cancelled() {
                            let _ = events.send(FlowEvent::RunCancelled { run });
                            return;
                        }
                        if let Err(e) = store
                            .write_slot(dst, value.clone(), WriteSlotOpts::live())
                            .await
                        {
                            tracing::warn!(
                                run = %run,
                                target = ?dst,
                                error = %e,
                                "propagator failed to fan out to downstream slot",
                            );
                        }
                    }
                }

                // 2. Trigger the node that owns the slot if the slot
                //    is in its triggering-input set.
                let triggers_node = topology
                    .triggers
                    .get(&env.slot.node)
                    .is_some_and(|inputs| inputs.contains(&env.slot.slot));
                if !triggers_node {
                    continue;
                }
                let Some(behavior) = topology.behaviors.get(&env.slot.node).cloned() else {
                    continue;
                };

                if cancel.is_cancelled() {
                    let _ = events.send(FlowEvent::RunCancelled { run });
                    return;
                }

                // Build the input map by reading every declared input
                // slot of the node from the graph store — the
                // propagator never owns slot data.
                let mut input: SlotMap = SlotMap::new();
                if let Some(inputs) = topology.triggers.get(&env.slot.node) {
                    for name in inputs {
                        let sr = SlotRef::new(env.slot.node.clone(), name.clone());
                        if let Ok(v) = store.read_slot(&sr).await {
                            input.insert(name.clone(), v);
                        }
                    }
                }

                let node_id = env.slot.node.clone();
                let _ = events.send(FlowEvent::NodeStarted {
                    run,
                    node: node_id.clone(),
                });
                let ctx = NodeCtx::new(run, &node_id, &*cancel);
                let invoke_res = behavior.invoke(ctx, input).await;

                match invoke_res {
                    Ok(output) => {
                        for (slot_name, out_value) in output {
                            let _ = events.send(FlowEvent::NodeEmitted {
                                run,
                                node: node_id.clone(),
                                slot: slot_name.clone(),
                                value: out_value.clone(),
                            });
                            let sr = SlotRef::new(node_id.clone(), slot_name);
                            if let Err(e) = store
                                .write_slot(&sr, out_value, WriteSlotOpts::live())
                                .await
                            {
                                tracing::warn!(
                                    run = %run,
                                    node = %node_id,
                                    error = %e,
                                    "propagator failed to write node output",
                                );
                            }
                        }
                    }
                    Err(e) => {
                        let _ = events.send(FlowEvent::node_failed(run, node_id, &e));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::InMemoryGraphStore;
    use crate::run::RunCancel;

    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use async_trait::async_trait;
    use starter_flow_spi::node::{KindId, NodeError, SlotValue};
    use tokio::time::{sleep, timeout};

    fn nid(s: &str) -> NodeId {
        NodeId::new(s).unwrap()
    }

    fn slot(node: &str, name: &str) -> SlotRef {
        SlotRef::new(nid(node), name)
    }

    /// Behavior: identity on `in` → `out`. Counts invocations.
    struct Identity {
        kind: KindId,
        calls: Arc<AtomicU64>,
    }

    impl Identity {
        fn new() -> (Arc<Self>, Arc<AtomicU64>) {
            let calls = Arc::new(AtomicU64::new(0));
            let kind = KindId::new("starter.flow.test-identity").unwrap();
            (
                Arc::new(Self {
                    kind,
                    calls: calls.clone(),
                }),
                calls,
            )
        }
    }

    #[async_trait]
    impl NodeBehavior for Identity {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let v = input.get("in").cloned().unwrap_or(SlotValue::Null);
            let mut out = SlotMap::new();
            out.insert("out".to_owned(), v);
            Ok(out)
        }
    }

    /// Behavior: `out = in + 1` (always different from input — used to
    /// defeat the R3 short-circuit when chained in a cycle).
    struct Incrementer {
        kind: KindId,
        calls: Arc<AtomicU64>,
    }

    impl Incrementer {
        fn new() -> (Arc<Self>, Arc<AtomicU64>) {
            let calls = Arc::new(AtomicU64::new(0));
            let kind = KindId::new("starter.flow.test-incrementer").unwrap();
            (
                Arc::new(Self {
                    kind,
                    calls: calls.clone(),
                }),
                calls,
            )
        }
    }

    #[async_trait]
    impl NodeBehavior for Incrementer {
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
            out.insert("out".to_owned(), SlotValue::Int(n + 1));
            Ok(out)
        }
    }

    /// Drain a broadcast receiver into a Vec (best-effort). Skips
    /// `Lagged` errors (when a high-frequency producer overruns the
    /// receiver's buffer) so the drain continues to the tail of the
    /// stream, which is what the assertions in this module care
    /// about.
    async fn drain_events(rx: &mut broadcast::Receiver<FlowEvent>) -> Vec<FlowEvent> {
        let mut out = Vec::new();
        loop {
            match timeout(Duration::from_millis(50), rx.recv()).await {
                Ok(Ok(ev)) => out.push(ev),
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => break,
                Err(_) => break,
            }
        }
        out
    }

    /// A → B → C: a value seeded on `a.out` flows through two
    /// invocations and ends up on `c.out`.
    #[tokio::test]
    async fn linear_three_node_chain_propagates_end_to_end() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

        let (b_behavior, b_calls) = Identity::new();
        let (c_behavior, c_calls) = Identity::new();

        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.a", "out"), vec![slot("flow.test.b", "in")]);
        links.insert(slot("flow.test.b", "out"), vec![slot("flow.test.c", "in")]);

        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.b"), BTreeSet::from(["in".to_owned()]));
        triggers.insert(nid("flow.test.c"), BTreeSet::from(["in".to_owned()]));

        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.b"), b_behavior);
        behaviors.insert(nid("flow.test.c"), c_behavior);

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let (tx, mut rx) = broadcast::channel::<FlowEvent>(64);
        let cancel = RunCancel::new();
        let run = RunId::new();
        let handle = spawn(
            store.clone(),
            topology,
            cancel.clone(),
            tx.clone(),
            run,
            PropagatorConfig::default(),
        );

        // Seed the chain by writing the source slot.
        store
            .write_slot(
                &slot("flow.test.a", "out"),
                SlotValue::Int(42),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();

        // Wait for c.out to land.
        let mut final_value = None;
        for _ in 0..50 {
            sleep(Duration::from_millis(20)).await;
            if let Ok(v) = store.read_slot(&slot("flow.test.c", "out")).await {
                final_value = Some(v);
                break;
            }
        }
        assert_eq!(final_value, Some(SlotValue::Int(42)));
        assert_eq!(b_calls.load(Ordering::SeqCst), 1);
        assert_eq!(c_calls.load(Ordering::SeqCst), 1);

        // Shut down cleanly.
        cancel.cancel();
        let _ = timeout(Duration::from_millis(500), handle).await;
        let _events = drain_events(&mut rx).await;
    }

    /// Self-loop with an identity behavior: the second copy back into
    /// `n.in` sees the same value and hits the R3 idempotent-write
    /// short-circuit at the store. Propagation terminates without
    /// hitting the hop budget.
    #[tokio::test]
    async fn cycle_terminates_on_idempotent_short_circuit() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

        let (n_behavior, n_calls) = Identity::new();

        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.n", "out"), vec![slot("flow.test.n", "in")]);

        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.n"), BTreeSet::from(["in".to_owned()]));

        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.n"), n_behavior);

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let (tx, mut rx) = broadcast::channel::<FlowEvent>(64);
        let cancel = RunCancel::new();
        let run = RunId::new();
        let handle = spawn(
            store.clone(),
            topology,
            cancel.clone(),
            tx.clone(),
            run,
            PropagatorConfig::default(),
        );

        store
            .write_slot(
                &slot("flow.test.n", "in"),
                SlotValue::Int(7),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();

        // Give propagation time to settle. With identity + R3 we expect
        // exactly one invocation: the second-round write to n.in carries
        // the same value as the seed and gets short-circuited.
        sleep(Duration::from_millis(200)).await;
        assert_eq!(
            n_calls.load(Ordering::SeqCst),
            1,
            "identity self-loop must terminate after one invocation",
        );

        // No `RunFailed { CycleBudgetExhausted }` event.
        cancel.cancel();
        let _ = timeout(Duration::from_millis(500), handle).await;
        let events = drain_events(&mut rx).await;
        let has_budget_failure = events.iter().any(|e| {
            matches!(
                e,
                FlowEvent::RunFailed { error, .. } if error.contains("cycle budget")
            )
        });
        assert!(
            !has_budget_failure,
            "idempotent self-loop must not fail the run on cycle budget"
        );
    }

    /// Self-loop with an Incrementer always produces a new value, so
    /// the R3 short-circuit never fires. The propagator's own
    /// `max_propagation_hops` budget must catch the run.
    #[tokio::test]
    async fn forced_no_shortcut_cycle_terminates_on_max_propagation_hops() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

        let (n_behavior, _n_calls) = Incrementer::new();

        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.n", "out"), vec![slot("flow.test.n", "in")]);

        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.n"), BTreeSet::from(["in".to_owned()]));

        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.n"), n_behavior);

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let (tx, mut rx) = broadcast::channel::<FlowEvent>(256);
        let cancel = RunCancel::new();
        let run = RunId::new();
        let cfg = PropagatorConfig {
            max_propagation_hops: 10,
        };
        let handle = spawn(
            store.clone(),
            topology,
            cancel.clone(),
            tx.clone(),
            run,
            cfg,
        );

        store
            .write_slot(
                &slot("flow.test.n", "in"),
                SlotValue::Int(1),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();

        // Wait for the propagator task to exit on its own (budget hit).
        let join_res = timeout(Duration::from_secs(2), handle).await;
        assert!(
            join_res.is_ok(),
            "propagator must terminate on cycle budget"
        );

        let events = drain_events(&mut rx).await;
        let found_budget_failure = events.iter().any(|e| {
            matches!(
                e,
                FlowEvent::RunFailed { error, .. } if error.contains("cycle budget")
            )
        });
        assert!(
            found_budget_failure,
            "expected RunFailed(CycleBudgetExhausted); got: {events:?}",
        );

        // We did not have to fire cancel — the propagator self-terminated.
        assert!(!cancel.is_cancelled());
    }

    /// A fired Cancel mid-run stops the propagator scheduling further
    /// hops within a few hundred milliseconds.
    #[tokio::test]
    async fn cancel_mid_run_stops_scheduling() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

        let (n_behavior, n_calls) = Incrementer::new();

        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.n", "out"), vec![slot("flow.test.n", "in")]);

        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.n"), BTreeSet::from(["in".to_owned()]));

        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.n"), n_behavior);

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let (tx, mut rx) = broadcast::channel::<FlowEvent>(4096);
        let cancel = RunCancel::new();
        let run = RunId::new();
        // Generous budget so the loop would otherwise run a long time.
        let cfg = PropagatorConfig {
            max_propagation_hops: 1_000_000,
        };
        let handle = spawn(
            store.clone(),
            topology,
            cancel.clone(),
            tx.clone(),
            run,
            cfg,
        );

        store
            .write_slot(
                &slot("flow.test.n", "in"),
                SlotValue::Int(1),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();

        // Let propagation run briefly, then cancel.
        sleep(Duration::from_millis(50)).await;
        cancel.cancel();

        // The propagator should exit promptly.
        let join_res = timeout(Duration::from_millis(500), handle).await;
        assert!(
            join_res.is_ok(),
            "propagator did not stop within 500ms of cancel",
        );

        // After cancel + a settling window, no further invocations.
        let calls_after_cancel = n_calls.load(Ordering::SeqCst);
        sleep(Duration::from_millis(200)).await;
        let calls_settled = n_calls.load(Ordering::SeqCst);
        assert_eq!(
            calls_after_cancel, calls_settled,
            "propagator scheduled more work after cancel",
        );

        // RunCancelled was emitted.
        let events = drain_events(&mut rx).await;
        let saw_cancelled = events
            .iter()
            .any(|e| matches!(e, FlowEvent::RunCancelled { .. }));
        assert!(
            saw_cancelled,
            "expected RunCancelled in event stream; got: {events:?}",
        );
    }
}
