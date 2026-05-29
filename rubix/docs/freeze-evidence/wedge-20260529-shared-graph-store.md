# Wedge root cause: shared flow graph store (2026-05-29)

**Status: definitive.** Supersedes the two earlier diagnoses recorded in
[`rubix/crates/rubix-watchdog/README.md`](../../crates/rubix-watchdog/README.md)
(the `graph.rs` span-guard theory and the dashboard-SSE auth-pool-leak
theory). Both were real footguns but **neither was the cause of the
runtime freeze** — the agent kept wedging on the same signature after
both "fixes" landed.

---

## TL;DR

Every flow run executed against a **single shared `InMemoryGraphStore`**
owned by the engine. Concurrent runs (overlapping scheduler fires, and —
in production — dashboard-driven runs) write the **same slot refs** and
subscribe to the **same `SlotChanged` broadcast**, so each run's
propagator keeps receiving the *other* runs' events and **never reaches
quiescence**. Runs therefore **never terminate**; their propagator /
coordinator tasks and broadcast receivers **accumulate** until the shared
`nodes` `RwLock` / broadcast collapses the runtime — every tokio worker
parks on a futex, no worker is left to drive the I/O reactor, `/livez`
stops answering, and the frontend can no longer reach the backend.

**Fix:** give each run its own `InMemoryGraphStore`. Applied in both
flow-run entry points in
[`crates/starter-flow-surfaces/src/lib.rs`](../../../crates/starter-flow-surfaces/src/lib.rs).

---

## Symptom & `/proc` signature (live capture, 2026-05-29)

Captured from the live wedged process before it was recycled:

- Boot `06:32:35Z`, froze `06:48:35Z` — **16 min uptime**, dead-centre in
  the documented 14–18 min MTBF.
- **33 threads, every one `State: S` parked on `futex_do_wait`.** No
  `epoll_wait` anywhere.
- **0 voluntary context switches across all 33 threads over a 1.2 s
  sample** → total runtime deadlock, not mere overload.
- Port 8088 still `LISTEN` with **`Recv-Q=12`** — twelve connections
  stranded in the accept backlog the dead runtime cannot drain. *This is
  the direct mechanism by which "the frontend can't reach the backend".*
- The log **stopped mid-telemetry-cycle** (logged `rubix-mcp` +
  `warehouse` pool stats, then nothing) — the classic "log simply stops
  growing" freeze.
- **No `rubix-watchdog` process was running**, so nothing auto-recovered
  it. Arming `make watchdog-bg` is the operational backstop (see README).

## Why the two prior diagnoses were wrong/incomplete

1. **Dashboard-SSE auth-pool leak (README's "real root cause").** The
   dedicated `rubix-dash-listen` pool fix *held*: at the 2026-05-29
   freeze the `rubix-auth` pool was healthy (`size=6 idle=6 in_use=0`,
   `max=16`) and there was **no** `pool near saturation` warning anywhere
   in the log. The runtime wedged anyway. The pool leak was real but
   secondary.
2. **`graph.rs` span-guard theory.** Already recanted in the README; the
   committed code is correct (`write_slot` uses `.instrument()`,
   `write_slot_batch` never enters its spans). Not the cause.
3. **"Duplicate cron / two schedulers" (compounding factor #2).** Moot.
   `starter.flow.trigger.schedule` is a **passive** node
   ([`crates/starter-flow-nodes/src/trigger_schedule.rs`](../../../crates/starter-flow-nodes/src/trigger_schedule.rs):13)
   — it does not self-fire, and its `cron_expr` is **required node
   config** the body reads, not a second scheduler. Do not "dedup" it;
   removing it breaks the node.

## Root cause in detail

The engine is built with **one** `InMemoryGraphStore`:

- [`rubix/crates/rubix-agent/src/boot/mcp/mod.rs`](../../crates/rubix-agent/src/boot/mcp/mod.rs)
  `build_engine` → `let graph_store = Arc::new(InMemoryGraphStore::new()); Engine::new(graph_store)`.

Every flow ran against that shared store:

- `FlowAsTool::invoke_with_cancel_inner` → `FlowRunner::new(self.engine.store.clone(), …)`
- `FlowAsServiceWorkerHandle::handle_event` → `FlowRunner::new(self.engine.store.clone(), …)`

(Note the asymmetry that hid the bug: the *run* store was already
per-invocation (`InMemoryRunStore::new()`), but the *graph* store — where
slot values **and the `SlotChanged` broadcast** live — was shared.)

The propagator subscribes to the store's broadcast
([`crates/starter-flow/src/propagator.rs`](../../../crates/starter-flow/src/propagator.rs):375)
and only exits on cancel / cycle-budget / stream-close. The shared
broadcast never closes, so:

1. Two runs of the same flow (or any flows sharing node ids) write the
   **same** `SlotRef`s into the one store.
2. Run A's `SlotChanged` is delivered to **both** A's and B's propagator;
   B re-fans it downstream → more events → delivered to all live
   propagators. O(N²) in concurrent runs.
3. No propagator goes quiescent while any other run is active, so
   `FlowAsTool`'s `handle.join`
   ([`lib.rs`](../../../crates/starter-flow-surfaces/src/lib.rs)) never
   returns → the invocation never completes → every tick adds more
   non-terminating runs + tasks + receivers.
4. The shared `nodes` `RwLock` / broadcast saturates; workers pile up and
   park on the futex; the I/O reactor stops being polled; total freeze.

This is consistent with the README's own tokio-console capture:
**142 → 552 alive tasks**, **70+ waiters on a single resource**.

The live 2026-05-29 log shows the fingerprint directly: at each tick,
**two `run_id`s emit the same counter value simultaneously** (overlapping
runs cross-firing on the shared store).

## The fix

Per-run `InMemoryGraphStore` in **both** flow-run paths in
[`crates/starter-flow-surfaces/src/lib.rs`](../../../crates/starter-flow-surfaces/src/lib.rs):

```rust
// before:  FlowRunner::new(self.engine.store.clone(), …)
let run_store = Arc::new(InMemoryGraphStore::new());
let mut runner = FlowRunner::new(run_store.clone(), …);
// …and the terminal-slot read-back reads from run_store, not engine.store.
```

Kept shared on purpose:

- **`NodeStateStore`** — durable per-node state (e.g. the tick-counter
  value) must persist across runs. Threaded in separately.
- **`FlowEventSink`** (the `FlowSubscriptionRegistry`) — the SSE
  fan-out at `/api/v1/flows/{id}/events`. Independent of the graph store.

Nothing outside a flow run reads the engine's flow store — the dashboard
/ SDUI live values use a **separate** `dashboard_graph` /
`RubixEntityGraph` ([`registry.rs`](../../crates/rubix-agent/src/registry.rs):164),
so per-run isolation is safe.

## Reproduction & verification

**Deterministic unit reproduction (the wedge in microcosm).** New test
`concurrent_invocations_are_isolated_per_run` in
[`crates/starter-flow-surfaces/tests/stage7_flow_as_tool.rs`](../../../crates/starter-flow-surfaces/tests/stage7_flow_as_tool.rs):
8 invocations launched concurrently through a `tokio::sync::Barrier` so
all node bodies are mid-flight at once.

- On a **shared** store the 8 runs **hang for 10 s (timeout)** — the
  deadlock reproduced deterministically.
- On **per-run** stores they complete and each observes only its own
  value.

(Verified both directions: reverting the fix makes the test hang/fail;
restoring it passes. All other surfaces tests stay green.)

**System-level A/B.** Relaunched the agent under gdb (agent as gdb's
child, because `ptrace_scope=1` + the apport `core_pattern` block
attaching to / core-dumping a non-descendant pid) with an accelerated
config `scheduler.tick_interval_seconds = 1` (default `60` throttles the
every-5s `*/5 * * * * *` crons to ~1/min). Post-fix over a 10-min accel
run: **each counter value emitted exactly once** (pre-fix showed
duplicates), `/livez` 200 throughout, no freeze.

## Open follow-up (NOT the freeze cause)

A **slow, ~2/min, time-based task drift** persists post-fix in the
headless agent (`num_alive_tasks` 23 → 43 over 10 min, no plateau). It is
**independent of flow firing rate** (the 60s-tick, 1s-tick, and post-fix
runs all drift ~2/min), so it is *not* the flow cross-talk and *not* the
16-min freeze (it would take ~4 h to reach wedge level). It needs
`make CONSOLE=1 restart` + tokio-console to attribute. Tracked as a
separate leak.

Also still open from the README's list: the `PgListenTail` drop latency,
the SSE subscription rate, and auditing other
`PgListener::connect_with(shared_pool)` sites.

## Files

- Fix: [`crates/starter-flow-surfaces/src/lib.rs`](../../../crates/starter-flow-surfaces/src/lib.rs)
  (`invoke_with_cancel_inner`, `handle_event`).
- Regression test: [`crates/starter-flow-surfaces/tests/stage7_flow_as_tool.rs`](../../../crates/starter-flow-surfaces/tests/stage7_flow_as_tool.rs)
  (`concurrent_invocations_are_isolated_per_run`).
- Engine wiring (the shared store): [`rubix/crates/rubix-agent/src/boot/mcp/mod.rs`](../../crates/rubix-agent/src/boot/mcp/mod.rs).
