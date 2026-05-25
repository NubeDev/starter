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

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use starter_flow_spi::flow::{FlowError, FlowEvent, RunId, RunState as SpiRunState, RunStore};
use starter_flow_spi::graph::{GraphStore, SubscribeOpts, SubscriptionStream, WriteSlotOpts};
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::state::{NodeStateStore, NoopNodeStateStore};
use starter_flow_spi::Cancel;

use crate::health::HealthHandle;
use crate::metrics::RunMetricsCell;

/// Default per-run propagation budget — locked at Phase 2 stage 1
/// "Decisions made" (R1 cycle-bound budget defaults).
pub const DEFAULT_MAX_PROPAGATION_HOPS: u64 = 1000;

/// Per-run monotonic tick counter.
///
/// SCOPE Phase 3 stage 6 / D-F3.11 long-uptime invariant: the
/// propagator's per-run hop counter must be a `u64` so it does not
/// wrap under years of uninterrupted ticking (`u64` at 1 kHz wraps
/// in ~584 million years). [`TickCounter`] is a one-field newtype
/// over `u64` so the underlying width is fixed at the type level;
/// the compile-time [`assert!`] below catches any accidental
/// re-typing on the future propagator refactor that splits this
/// counter out of the loop.
///
/// The counter starts at `0` and increments by one on every
/// `SlotChanged` event the propagator consumes. The current value
/// composes with the resume path's `initial_seq` (loaded from the
/// last checkpoint via [`CheckpointHook::initial_seq`]) to produce
/// the checkpoint's `seq` field: `seq = initial_seq + tick.get()`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TickCounter(u64);

impl TickCounter {
    /// Construct a fresh counter at zero.
    pub const fn new() -> Self {
        Self(0)
    }

    /// Increment by one (saturating at `u64::MAX`) and return the
    /// new value.
    pub fn tick(&mut self) -> u64 {
        self.0 = self.0.saturating_add(1);
        self.0
    }

    /// Borrow the current counter value.
    pub fn get(self) -> u64 {
        self.0
    }
}

// D-F3.11 long-uptime invariant: the tick counter is a `u64`,
// no wider, no narrower. Catching a refactor that widens / narrows
// this at compile time is cheaper than catching it in a soak.
const _: () = assert!(std::mem::size_of::<TickCounter>() == 8);

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

/// Backoff schedule for `RunStore::checkpoint` retries (D-F3.11).
/// 50 → 100 → 200 → 400 → 800 milliseconds, exactly 5 attempts.
/// The retry loop emits one [`FlowEvent::CheckpointFailed`] per
/// attempt; after the fifth the engine transitions to
/// [`starter_flow_spi::flow::EngineHealth::Degraded`] and the batch
/// is moved into the per-run in-memory queue.
pub const CHECKPOINT_BACKOFF_MS: [u64; 5] = [50, 100, 200, 400, 800];

/// One queued checkpoint batch the propagator was unable to
/// persist; held in memory while
/// [`starter_flow_spi::flow::EngineHealth::Degraded`] and drained on
/// the next successful `RunStore::checkpoint`.
#[derive(Debug, Clone)]
struct QueuedBatch {
    seq: u64,
    state: SpiRunState,
    writes: Vec<(SlotRef, SlotValue)>,
}

/// Per-run in-memory checkpoint queue (D-F3.11).
///
/// Capped at [`Self::capacity`]; once full, pushing evicts the
/// oldest batch and increments
/// [`RunMetricsCell::degraded_dropped_count`]. The queue drains in
/// `(run_id, seq)` order on the next successful `RunStore::checkpoint`
/// write; the queue is per-run so the FIFO insertion order is the
/// `seq` order (the propagator's `TickCounter` increments
/// monotonically).
#[derive(Debug)]
pub struct DegradedQueue {
    capacity: usize,
    inner: Mutex<VecDeque<QueuedBatch>>,
    metrics: Arc<RunMetricsCell>,
}

impl DegradedQueue {
    /// Construct an empty queue with the given capacity. `0` is
    /// treated as `1` so the engine never silently swallows every
    /// queued batch — D-F3.11 explicitly defaults the cap to 1024
    /// and the runner enforces a sane minimum.
    pub fn new(capacity: usize, metrics: Arc<RunMetricsCell>) -> Arc<Self> {
        Arc::new(Self {
            capacity: capacity.max(1),
            inner: Mutex::new(VecDeque::new()),
            metrics,
        })
    }

    fn push(&self, batch: QueuedBatch) {
        let mut q = self.inner.lock().expect("DegradedQueue mutex poisoned");
        while q.len() >= self.capacity {
            q.pop_front();
            self.metrics.add_dropped(1);
        }
        q.push_back(batch);
    }

    fn pop_front(&self) -> Option<QueuedBatch> {
        self.inner
            .lock()
            .expect("DegradedQueue mutex poisoned")
            .pop_front()
    }

    /// Snapshot count (test convenience).
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("DegradedQueue mutex poisoned")
            .len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Per-tick checkpoint hook the propagator calls at the end of every
/// event-handler iteration (D-F3.2: per-tick cadence, not per
/// `write_slot`).
///
/// Constructed by [`crate::run::FlowRunner`] when a Phase 3 SPI
/// [`RunStore`] is attached to the engine and threaded into
/// [`spawn_with_checkpoint`] / [`drive_with_checkpoint`].
///
/// Stage 6 (D-F3.11) fields:
///
/// - [`Self::health`] — shared engine-level health flag. The retry
///   loop flips this to [`starter_flow_spi::flow::EngineHealth::Degraded`]
///   after five consecutive failures, and back to `Healthy` once
///   the next checkpoint succeeds and the queue is drained.
/// - [`Self::queue`] — per-run in-memory queue used to hold
///   checkpoint batches when the backend is degraded. Drained in
///   `(run_id, seq)` order on the next successful write.
/// - [`Self::metrics`] — per-run live counters; the queue increments
///   `degraded_dropped_count` on overflow.
///
/// `initial_seq` is the propagator-tick counter the resume path
/// loaded from `RunStore::load(run_id)`; a fresh run passes `0`.
#[derive(Clone)]
#[non_exhaustive]
pub struct CheckpointHook {
    /// The SPI [`RunStore`] every per-tick checkpoint writes to.
    pub store: Arc<dyn RunStore>,
    /// Seq offset applied to the propagator's per-run hop counter
    /// when computing the checkpoint `seq` field. `0` for a fresh
    /// run; loaded checkpoint's `seq` for a resumed run.
    pub initial_seq: u64,
    /// Engine-level health flag the retry loop flips on degrade /
    /// recovery (D-F3.11).
    pub health: HealthHandle,
    /// Per-run in-memory queue for degraded-mode batches (D-F3.11).
    pub queue: Arc<DegradedQueue>,
    /// Per-run live metrics counters (D-F3.10 + D-F3.11).
    pub metrics: Arc<RunMetricsCell>,
}

impl CheckpointHook {
    /// Construct a [`CheckpointHook`] with the stage-6 durability
    /// fields fully wired. Sized for use by [`crate::run::FlowRunner`]
    /// at run launch time.
    pub fn new(
        store: Arc<dyn RunStore>,
        initial_seq: u64,
        health: HealthHandle,
        queue: Arc<DegradedQueue>,
        metrics: Arc<RunMetricsCell>,
    ) -> Self {
        Self {
            store,
            initial_seq,
            health,
            queue,
            metrics,
        }
    }
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
    spawn_with_checkpoint(
        store,
        topology,
        cancel,
        events,
        run,
        config,
        None,
        Arc::new(SkillSelection::None),
        Arc::new(NoopNodeStateStore),
        None,
    )
}

/// Same as [`spawn`] but with an optional [`CheckpointHook`] for
/// per-tick `RunStore::checkpoint` persistence (D-F3.2). When
/// `checkpoint` is `None` the propagator behaves identically to
/// [`spawn`] — the Phase-2 in-memory substrate.
#[allow(clippy::too_many_arguments)]
pub fn spawn_with_checkpoint(
    store: Arc<dyn GraphStore>,
    topology: Arc<FlowTopology>,
    cancel: Arc<dyn Cancel>,
    events: broadcast::Sender<FlowEvent>,
    run: RunId,
    config: PropagatorConfig,
    checkpoint: Option<CheckpointHook>,
    skill: Arc<SkillSelection>,
    node_state: Arc<dyn NodeStateStore>,
    flow_id: Option<starter_flow_spi::flow::FlowId>,
) -> JoinHandle<()> {
    // Subscribe *synchronously* before spawning the task so that any
    // writes the caller performs immediately after `spawn()` returns
    // are guaranteed to land in the propagator's subscription queue
    // — no "did the spawned task get to `subscribe` first?" races.
    let sub = store.subscribe(SubscribeOpts::default());
    tokio::spawn(drive_with_checkpoint(
        store, sub, topology, cancel, events, run, config, checkpoint, skill, node_state, flow_id,
    ))
}

/// Body of the propagator task. Public for ergonomics ([`spawn`] is
/// the recommended entry point) but kept on a stable surface so smoke
/// tests and Phase 3 engine wiring can drive the loop inline.
pub async fn drive(
    store: Arc<dyn GraphStore>,
    sub: SubscriptionStream,
    topology: Arc<FlowTopology>,
    cancel: Arc<dyn Cancel>,
    events: broadcast::Sender<FlowEvent>,
    run: RunId,
    config: PropagatorConfig,
) {
    drive_with_checkpoint(
        store,
        sub,
        topology,
        cancel,
        events,
        run,
        config,
        None,
        Arc::new(SkillSelection::None),
        Arc::new(NoopNodeStateStore),
        None,
    )
    .await
}

/// Same as [`drive`] but with an optional per-tick checkpoint hook.
#[allow(clippy::too_many_arguments)]
pub async fn drive_with_checkpoint(
    store: Arc<dyn GraphStore>,
    mut sub: SubscriptionStream,
    topology: Arc<FlowTopology>,
    cancel: Arc<dyn Cancel>,
    events: broadcast::Sender<FlowEvent>,
    run: RunId,
    config: PropagatorConfig,
    checkpoint: Option<CheckpointHook>,
    skill: Arc<SkillSelection>,
    node_state: Arc<dyn NodeStateStore>,
    flow_id: Option<starter_flow_spi::flow::FlowId>,
) {
    let mut tick = TickCounter::new();

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

                let hops = tick.tick();
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

                // Per-tick checkpoint batch (D-F3.2): every `write_slot`
                // call this iteration issues is mirrored into
                // `tick_writes` and persisted in one
                // `RunStore::checkpoint(...)` call at the end. The
                // batch lives only when a `CheckpointHook` is
                // attached so the no-store path allocates nothing.
                let mut tick_writes: Vec<(SlotRef, SlotValue)> = Vec::new();

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
                        } else if checkpoint.is_some() {
                            tick_writes.push((dst.clone(), value.clone()));
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
                    // Per-tick checkpoint even when the slot only
                    // fans out without triggering — the writes still
                    // need to durably land before the next tick.
                    if let Some(hook) = checkpoint.as_ref() {
                        if !tick_writes.is_empty() {
                            let seq = hook.initial_seq.saturating_add(hops);
                            checkpoint_one_tick(
                                hook,
                                run,
                                seq,
                                SpiRunState::Running,
                                tick_writes,
                                &events,
                            )
                            .await;
                        }
                    }
                    continue;
                }
                let Some(behavior) = topology.behaviors.get(&env.slot.node).cloned() else {
                    if let Some(hook) = checkpoint.as_ref() {
                        if !tick_writes.is_empty() {
                            let seq = hook.initial_seq.saturating_add(hops);
                            checkpoint_one_tick(
                                hook,
                                run,
                                seq,
                                SpiRunState::Running,
                                tick_writes,
                                &events,
                            )
                            .await;
                        }
                    }
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
                // Thread the owning flow id into `NodeCtx` when the
                // caller supplied one so stateful node kinds (e.g.
                // `starter.flow.counter`) can build a `NodeStateKey`.
                let ctx = if let Some(flow) = flow_id.as_ref() {
                    NodeCtx::with_flow(flow, run, &node_id, &*cancel, &skill, &*node_state)
                } else {
                    NodeCtx::new(run, &node_id, &*cancel, &skill, &*node_state)
                };
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
                                .write_slot(&sr, out_value.clone(), WriteSlotOpts::live())
                                .await
                            {
                                tracing::warn!(
                                    run = %run,
                                    node = %node_id,
                                    error = %e,
                                    "propagator failed to write node output",
                                );
                            } else if checkpoint.is_some() {
                                tick_writes.push((sr, out_value));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = events.send(FlowEvent::node_failed(run, node_id, &e));
                    }
                }

                // Per-tick checkpoint (D-F3.2). Single call per
                // propagator iteration, regardless of fan-out width.
                // Stage 6 (D-F3.11) wraps the call in retry-with-
                // backoff and the degraded-mode queue; see
                // `checkpoint_one_tick` for the policy.
                if let Some(hook) = checkpoint.as_ref() {
                    if !tick_writes.is_empty() {
                        let seq = hook.initial_seq.saturating_add(hops);
                        checkpoint_one_tick(
                            hook,
                            run,
                            seq,
                            SpiRunState::Running,
                            tick_writes,
                            &events,
                        )
                        .await;
                    }
                }
            }
        }
    }
}

/// Attempt to persist one per-tick checkpoint with retry-with-backoff
/// (D-F3.11), and on success drain any batches the degraded queue
/// accumulated while the backend was unreachable.
///
/// Behaviour:
///
/// 1. Try `hook.store.checkpoint(...)` for up to
///    [`CHECKPOINT_BACKOFF_MS`]`.len()` attempts. On each failure,
///    emit [`FlowEvent::CheckpointFailed`] with the 1-indexed
///    `attempt` and sleep the backoff window before the next try.
/// 2. On success: flip the engine back to
///    [`starter_flow_spi::flow::EngineHealth::Healthy`], then drain
///    the per-run queue in `(run_id, seq)` order. A drain entry
///    that itself fails goes back on the queue head and aborts the
///    drain — the engine returns to `Degraded` on its next failure.
/// 3. After all attempts fail: flip to
///    [`starter_flow_spi::flow::EngineHealth::Degraded`] and push the
///    current batch onto the per-run queue (evict-oldest on overflow
///    increments [`crate::metrics::RunMetricsCell::degraded_dropped_count`]).
async fn checkpoint_one_tick(
    hook: &CheckpointHook,
    run: RunId,
    seq: u64,
    state: SpiRunState,
    writes: Vec<(SlotRef, SlotValue)>,
    events: &broadcast::Sender<FlowEvent>,
) {
    match try_persist_with_backoff(hook, run, seq, state, &writes, events).await {
        Ok(()) => {
            // Successful write → engine is healthy again. Drain the
            // per-run queue in FIFO (== seq) order.
            hook.health.set_healthy();
            while let Some(batch) = hook.queue.pop_front() {
                let drain_writes = batch.writes;
                let drain_seq = batch.seq;
                let drain_state = batch.state;
                if let Err(()) = try_persist_with_backoff(
                    hook,
                    run,
                    drain_seq,
                    drain_state,
                    &drain_writes,
                    events,
                )
                .await
                {
                    // Drain failure: push back at head, mark
                    // degraded, abort drain. The next successful
                    // tick will pick up here.
                    hook.health.set_degraded();
                    hook.queue.push(QueuedBatch {
                        seq: drain_seq,
                        state: drain_state,
                        writes: drain_writes,
                    });
                    return;
                }
            }
        }
        Err(()) => {
            // All retries exhausted → degrade + enqueue.
            hook.health.set_degraded();
            hook.queue.push(QueuedBatch { seq, state, writes });
        }
    }
}

/// Best-effort persist with the [`CHECKPOINT_BACKOFF_MS`] schedule.
/// Returns `Ok(())` on the first successful attempt, `Err(())` if
/// every attempt fails. Emits one [`FlowEvent::CheckpointFailed`]
/// per failed attempt.
async fn try_persist_with_backoff(
    hook: &CheckpointHook,
    run: RunId,
    seq: u64,
    state: SpiRunState,
    writes: &[(SlotRef, SlotValue)],
    events: &broadcast::Sender<FlowEvent>,
) -> Result<(), ()> {
    let max_attempts = CHECKPOINT_BACKOFF_MS.len() as u32;
    for attempt in 1..=max_attempts {
        match hook.store.checkpoint(run, seq, state, writes).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let error = e.to_string();
                tracing::warn!(
                    run = %run,
                    seq,
                    attempt,
                    error = %error,
                    "run_store checkpoint failed",
                );
                let _ = events.send(FlowEvent::CheckpointFailed {
                    run,
                    error,
                    attempt,
                });
                if attempt < max_attempts {
                    let backoff =
                        Duration::from_millis(CHECKPOINT_BACKOFF_MS[(attempt - 1) as usize]);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }
    Err(())
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
