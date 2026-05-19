# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 2 — REVIEW gate (Phase 3 + 24/7 durability boundary).**
Stage 1 (prose-only decision lock) is complete and pushed.
Do not advance to stage 3 until the user signs off.

## Stage 1 outcome (one-line summary)

D-F3.1 through D-F3.12 locked verbatim against the source-of-truth
flow SCOPE; Q1–Q8 resolved; **Q4 → STANDALONE** (no four-transport
smoke exists under `crates/smoke-tests/tests/` — five files there
are workspace-level invariants, none transport-stream); one
implementation-choice flag carried into stage 3
(`EventSink::dedup_key` signature: `(&self, &Event)` vs
`(&self, kind: &str, payload: &Value)` — both honour D-F3.12).

## What stage 2 reviewer needs to confirm

The six load-bearing decisions WORKFLOW.md flags (D-F3.1, D-F3.2,
D-F3.6, D-F3.8, D-F3.11, D-F3.12) plus Q4. Stage-3 through stage-9
re-do cost is high if these are wrong.

1. **D-F3.1 — SPI trait method shapes.** Async + `&self` +
   `Result<T, FlowError>`. `RunStore::checkpoint` takes
   `&[(SlotRef, SlotValue)]` (borrowed). `RunStore` gains
   `find_by_dedup_key(service_name, dedup_key)`. `FlowError`
   additively gains `NotFound { kind, id }`. Every new public
   enum + config struct `#[non_exhaustive]`.
2. **D-F3.2 — per-tick checkpoint cadence**, not per-write.
   Propagator's existing tick counter is `seq`.
3. **D-F3.6 — smokes live in `crates/smoke-tests/`** per the D1d
   revisit-trigger from the Phase 2 SCOPE. Engine-internal smokes
   stay where they are.
4. **D-F3.8 — `BEGIN IMMEDIATE` atomic-tx checkpoint + WAL pragmas**
   (`journal_mode=WAL, synchronous=NORMAL, busy_timeout=5000,
   foreign_keys=ON`) applied by extending the existing pool-init
   path.
5. **D-F3.11 — Degraded-mode backend-failure posture.** Retry-with-
   backoff 50→100→200→400→800ms capped at 5; after the 5th the
   engine transitions to `Degraded`; in-flight runs keep serving
   from in-memory state (queue bounded by
   `RunOpts.degraded_queue_capacity`, evict-oldest);
   `Engine::start` returns `BackendUnavailable` while Degraded;
   successful checkpoint drains queue and clears Degraded.
6. **D-F3.12 — at-least-once + dedup.** `EventSink::dedup_key()`
   additive optional method (default `None`); blake3 fallback;
   `UNIQUE (service_name, dedup_key)` partial index on `runs`;
   re-deliveries short-circuit via `find_by_dedup_key` and emit
   `FlowEvent::DedupShortCircuit`.

## Reality-check addenda the reviewer should see

These came out of stage-1 verification against the live workspace
and are recorded in SCOPE.md §"Stage 1 — decision lock":

- **`FlowEvent::SlotChanged` is a doc-only carry-over.** The
  per-run stream variant in `starter-flow-spi` is `NodeEmitted`;
  `GraphEvent::SlotChanged` is the internal graph-level bus
  name. Stage-6 + Stage-9 assertions use `NodeEmitted` for the
  "in-flight runs keep emitting during outage" invariant. No new
  `FlowEvent` variant added beyond `CheckpointFailed` +
  `DedupShortCircuit`.
- **`RunOpts` is net-new in stage 3.** Phase 2 has only
  `PropagatorConfig { max_propagation_hops }` in
  `starter-flow`. Stage 3 introduces `RunOpts` in
  `starter-flow-spi`; stage 5's `Engine::with_run_store(…)`
  builder projects `RunOpts.max_propagation_hops` into the
  existing `PropagatorConfig`. **No rename or refactor of
  `PropagatorConfig`** (out of scope per WORKFLOW anti-pattern).
- **`EventSink::dedup_key` signature choice deferred to stage 3
  implementation.** The Phase 2 `EventSink::emit(&self, kind:
  &str, payload: Value)` shape gives stage 3 two source-compatible
  options for the additive default-`None` accessor; pick at
  implementation time. Both honour D-F3.12.
- **Four Phase 3 smoke files live as siblings of the existing
  five workspace smokes** under `crates/smoke-tests/tests/` with
  a `flow_*.rs` prefix so `ls` keeps ordering visible.

## Branch + commits

- Branch: `codeless/starter-flow-phase3-persistence-surfaces`.
- Stage 1 commit: `stage 1: lock Phase 3 + 24/7 durability
  boundary (D-F3.1..D-F3.12, Q1..Q8)`.
- Pushed to origin.

## What stage 3 starts with (after REVIEW passes)

- SPI trait fleshout in `crates/starter-flow-spi/src/flow.rs`
  (and possibly a new `session.rs`) per D-F3.1 + additive
  durability extensions per the Decisions block.
- Net-new `RunOpts` struct in `starter-flow-spi` (D-F3.1 + Q5
  addendum).
- `EventSink::dedup_key` additive method on the existing
  `starter-spi::service::sink::EventSink` trait (signature
  chosen at stage 3 implementation time per the reality-check
  addendum above).
- Baseline regenerated **in the same commit** per D-F3.7 even if
  byte-identical; commit message explicitly names the
  regeneration.
- Stages 4–10 producing a baseline diff is a stage-fail per
  D-F3.7; revert.

## Phase-3 follow-up notes (not in scope for this job)

- 1M-tick long-uptime soak as a nightly CI job (Q6).
- `RunOpts.checkpoint_backoff` if a consumer surfaces real need
  (Q8).
- `FlowEvent::HealthChanged` engine-level event once a
  Phase-7-owned engine-level event bus exists (Q7).
- `starter-store-postgres` `flow` feature mirror (D-F3.3 revisit
  trigger).
