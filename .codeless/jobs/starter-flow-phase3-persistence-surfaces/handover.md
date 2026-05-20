# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 8 — `FlowAsService` body in `starter-flow-surfaces`.**
Stage 7 (`FlowAsTool` body + builder + `Tool` impl wired
through the engine with health/cancel/forwarder/event-watcher
plumbing) is complete and pushed.

## Stages complete

- **Stage 1.** D-F3.1..D-F3.12 + Q1..Q8 locked; Q4 → STANDALONE.
- **Stage 2.** REVIEW gate passed.
- **Stage 3.** SPI trait fleshout + baseline regeneration.
- **Stage 4.** Three SQLite store impls behind default-off `flow`
  feature; 8 integration tests green; workspace gates clean.
- **Stage 5.** Engine wiring: `Engine::with_run_store(...)`
  builder, per-tick checkpoint cadence in propagator, resume-
  from-checkpoint on `FlowRunner::resume(...)`, R2 chokepoint
  integrity test (`stage5_resume_chokepoint.rs`) green.
- **Stage 7.** `FlowAsTool` body in
  `crates/starter-flow-surfaces/src/lib.rs`: explicit-field
  builder (D-F3.4), `Tool` impl forwarding `invoke` into a
  one-shot `FlowRunner` off `Arc<Engine>` sharing
  `engine.health_handle()` + optional `engine.run_store()`,
  cancel-forwarder task wiring an external `Arc<RunCancel>` into
  the per-run cancel within ~50 ms, event-watcher task
  capturing `FlowEvent::NodeFailed` so a per-node error
  surfaces as typed `SpiError::Internal` even though the
  engine quiesces as `Completed`, terminal-slot read-back via
  an explicit `OutputAdapter`, plus `invoke_with_cancel` and
  `invoke_with_timeout` convenience methods. Six unit tests in
  `tests/stage7_flow_as_tool.rs` cover builder validation /
  happy-path / typed-error / Degraded-rejection /
  cancel-within-200ms / no-task-leak.
- **Stage 6.** Durability hardening:
  retry-with-backoff (50→100→200→400→800ms, 5 attempts) on
  `RunStore::checkpoint` errors emitting one
  `FlowEvent::CheckpointFailed` per attempt 1..=5;
  `EngineHealth::{Healthy, Degraded}` accessor on `Engine`
  backed by an `AtomicU8` (lock-free) via a shared `HealthHandle`;
  per-run in-memory `DegradedQueue` capped by
  `RunOpts.degraded_queue_capacity` (default 1024, evict-oldest)
  drained in `(run_id, seq)` order on the next successful
  checkpoint write; `FlowRunner::start` now returns
  `Result<RunHandle, EngineError>` and rejects with
  `EngineError::BackendUnavailable` while `Degraded`;
  `RunOpts.event_broadcast_capacity` wired into `FlowRunner::launch`
  with an engine-owned `Lagged`-watcher subscriber incrementing
  `RunMetrics.subscriber_lagged_count`; `TickCounter` newtype
  promoted from the propagator's `hops: u64` local with a
  `const _: () = assert!(size_of::<TickCounter>() == 8)`
  compile-time check, composing additively with stage 5's
  `CheckpointHook.initial_seq`.

## Stage 6 outcome (one-line summary)

Five durability invariants land on `starter-flow` (retry-with-
backoff + `EngineHealth` + degraded queue + `start`-rejection
+ per-run broadcast capacity + Lagged-watcher) plus the
`TickCounter` newtype with the compile-time size assertion;
seven unit tests in `stage6_durability.rs` cover each invariant
in isolation, all three Phase 2 smokes + the stage-5 resume
chokepoint test stay green, workspace dep-tree gates +
`starter-flow-spi` baseline unchanged.

## Stage 6 implementation choices made

- **Shared `HealthHandle` cloneable through the engine and the
  runner.** `HealthHandle` wraps an `Arc<AtomicU8>` with
  `Healthy = 0` / `Degraded = 1`. The propagator's retry-with-
  backoff loop flips it via `set_degraded()` / `set_healthy()`;
  `FlowRunner::start` reads via `health.get()`. `Engine::health()
  -> EngineHealth` is the lock-free SPI-typed accessor; the engine
  owns one canonical handle and hands clones via
  `Engine::health_handle()` to `FlowRunner::with_health_handle(...)`.
  Reason: lets surfaces (stage 7/8) construct one engine per
  process but spawn many per-call runners that all see the same
  health flag, satisfying the SCOPE D-F3.11 "engine-level health
  state" contract without forcing the runner to embed the engine.
- **Retry-with-backoff lives inside the propagator, not the
  store.** `checkpoint_one_tick` wraps `try_persist_with_backoff`
  which loops `1..=5` over the `CHECKPOINT_BACKOFF_MS`
  `[50, 100, 200, 400, 800]` schedule, emitting one
  `FlowEvent::CheckpointFailed { run, error, attempt }` per
  failed attempt and sleeping the backoff between (not after) the
  last attempt. Reason: the SPI `RunStore::checkpoint` contract
  stays a single-shot fallible call; the retry policy is engine
  policy, not store policy (matches D-F3.11 wording "the engine
  retries with exponential backoff").
- **Per-run `DegradedQueue` lives on `CheckpointHook`.** `CheckpointHook`
  gains `health: HealthHandle`, `queue: Arc<DegradedQueue>`, and
  `metrics: Arc<RunMetricsCell>` fields plus a `new(...)` ctor;
  the queue is constructed in `FlowRunner::launch` with the
  per-run `RunOpts.degraded_queue_capacity`. Reason: per-run
  ownership matches the SCOPE "drain in `(run_id, seq)` order"
  contract — `(run_id, seq)` ordering trivially holds because
  the queue is per-run and the `TickCounter` is monotonic, so a
  single `VecDeque` push-back / pop-front is the queue-drain
  contract verbatim.
- **`FlowRunner::start` returns `Result<RunHandle, EngineError>`.**
  The Degraded-rejection check is a single `AtomicU8` load before
  any per-run allocation, so the runner sheds load cleanly when
  the backend is unreachable. `EngineError` is the
  SPI-defined `starter_flow_spi::flow::EngineError`
  (`BackendUnavailable` + `Flow(FlowError)`), not the engine
  crate's internal state-machine `EngineError` (the latter stays
  the R12 transition matrix type). This is a public API change
  — the four internal tests in `run.rs` plus the
  `stage5_resume_chokepoint.rs` test were updated with
  `.expect("start rejected")` at their call sites.
- **`spawn_lagged_watcher` is a `metrics`-module helper, not
  inline in the runner.** Factored out as
  `pub fn spawn_lagged_watcher(events_tx, metrics) -> JoinHandle<()>`
  so the per-run launch path can `drop(spawn_lagged_watcher(...))`
  to fire-and-forget, and so the stage-6 test can exercise the
  Lagged-counter path directly against a hand-rolled broadcast
  channel (the test doesn't need a full flow run to assert the
  metric increments).
- **`TickCounter` is a newtype over `u64`, not a renamed type
  alias.** Field is private, accessed via `tick()` / `get()`.
  The `const _: () = assert!(std::mem::size_of::<TickCounter>()
  == 8);` runs at compile time so any future refactor that
  widens / narrows the counter fails the build, not the soak.
  Composes additively with stage 5's `CheckpointHook.initial_seq`:
  the per-run checkpoint seq is computed as
  `hook.initial_seq.saturating_add(tick.get())`.
- **Per-tick checkpoint fires for fan-out-only ticks too.** The
  prior `tick_writes` path only checkpointed when a node was
  triggered; stage 6 adds the same checkpoint call along the
  fan-out-only paths (`!triggers_node` and "no behavior") because
  the writes still need to durably land before the next tick
  (D-F3.2 "per-tick batch"). Reason: a flow whose downstream
  link fans out without invoking a node still produces durable
  slot state; skipping the checkpoint there would leave a
  recoverable gap.
- **Long quiescence in flaky-store tests.** The default
  `FlowRunnerConfig.quiescence` of 100 ms is shorter than the
  retry backoff intervals (200 / 400 / 800 ms), so the coordinator
  pre-quiesces while the propagator is mid-`tokio::time::sleep`
  in the retry loop. Stage-6 tests that exercise the retry path
  build a `long_quiesce_config()` with a 3 s window so the
  coordinator stays alive long enough for the retry loop +
  queue drain to complete. Reason: this is a test-rig choice;
  the production default keeps the 100 ms quiescence (matches
  Phase 2). A follow-up consideration is whether to make the
  retry loop cancellation-aware (it currently isn't — `cancel`
  is checked only at the propagator's outer `select!`); deferred
  because the SCOPE doesn't require it and cancel-during-retry
  works fine in practice (the propagator finishes the current
  attempt then exits on the next `sub.next()` await).
- **`broadcast_cap` clamped to `>= 1`.** `RunOpts.event_broadcast_capacity`
  is a `usize` with a 1024 default; `FlowRunner::launch` applies
  `.max(1)` so a misconfigured `0` doesn't panic
  `broadcast::channel`. Same `.max(1)` on `DegradedQueue::new`
  for the cap.

## Stage 6 files touched

- `crates/starter-flow/src/lib.rs` — declare the new `health` +
  `metrics` modules.
- `crates/starter-flow/src/health.rs` — new file:
  `HealthHandle` (`Arc<AtomicU8>` wrapper) + accessors.
- `crates/starter-flow/src/metrics.rs` — new file:
  `RunMetricsCell` (two `AtomicU64`s + snapshot into the SPI
  `RunMetrics`) + `spawn_lagged_watcher` helper.
- `crates/starter-flow/src/engine.rs` — `Engine.health:
  HealthHandle` field, `Engine::health() -> EngineHealth`,
  `Engine::health_handle() -> HealthHandle`.
- `crates/starter-flow/src/propagator.rs` — `TickCounter` newtype
  + compile-time size assert + propagator loop rewritten to use
  it; `CHECKPOINT_BACKOFF_MS` constant; `QueuedBatch` +
  `DegradedQueue` types; `CheckpointHook` gains `health`, `queue`,
  `metrics` fields + `new(...)` ctor; `checkpoint_one_tick` +
  `try_persist_with_backoff` private helpers; per-tick checkpoint
  call added along the fan-out-only paths.
- `crates/starter-flow/src/run.rs` — `FlowRunner.health:
  HealthHandle` field + `with_health_handle` + `health_handle()`
  accessor; `FlowRunner::start` returns
  `Result<RunHandle, EngineError>` (rejects on `Degraded`);
  `launch` reads `RunOpts.event_broadcast_capacity` (not the
  fixed `FlowRunnerConfig.event_buffer`), constructs the per-run
  `DegradedQueue` + `RunMetricsCell`, spawns the
  `spawn_lagged_watcher` task, threads health + queue + metrics
  into `CheckpointHook::new`; `RunHandle` gains `metrics:
  Arc<RunMetricsCell>` field; `run_coordinator` signature gains
  the three handles.
- `crates/starter-flow/tests/stage6_durability.rs` — new file,
  seven tests covering each invariant in isolation
  (`tick_counter_is_u64_sized`,
  `engine_default_health_is_healthy_and_handle_round_trips`,
  `failing_run_store_emits_five_checkpoint_failed_events_then_degrades`,
  `start_rejects_with_backend_unavailable_while_degraded`,
  `degraded_queue_evict_oldest_increments_dropped_count`,
  `engine_recovers_to_healthy_when_store_comes_back`,
  `lagged_subscriber_increments_subscriber_lagged_count`).
- `crates/starter-flow/tests/stage5_resume_chokepoint.rs` —
  call-site update for `runner.start(...)` returning `Result`.

## Known pre-existing issues (NOT caused by stage 3/4/5/6)

- `cargo clippy --workspace --all-targets -- -D warnings` fails
  on master too with `error[E0432]: unresolved import
  'starter_grpc::testing'` in
  `crates/starter-grpc/tests/tools_service.rs` — needs
  `--features testing` which workspace clippy doesn't add.
- `cargo fmt --check` reports pre-existing drift in:
  `crates/starter-spi/src/ui/theme/mod.rs`,
  `crates/starter-ui-theme/src/{lib,routes}.rs`,
  `examples/notes/src/server.rs`, and several files under
  `starter-extensions/`.
- All four CI red checks on PR #9 (`rust check`, `pnpm
  build/typecheck`, `starter-spi dep baseline`, `openapi/ts
  drift`) also fail on master and are not caused by this
  branch.
- `cargo` emits `default-features = false` warnings for
  `starter-flow-spi` / `starter-spi` workspace deps —
  pre-existing on master in `starter-flow-surfaces` and
  `starter-flow-nodes`. Out of scope.

## Branch + commits

- Branch: `codeless/starter-flow-phase3-persistence-surfaces`.
- Stage 1 commit: `8407d6e` — decision lock.
- Stage 3 commit: `44f19f5` — SPI fleshout + baseline regeneration.
- Stage 4 commit: `8a60ddb` — SQLite store impls.
- Stage 5 commit: `f8fed76` — engine wiring + resume + R2
  chokepoint test.
- Stage 6 commit: see latest log — durability hardening.
- Pushed to origin.

## Stage 7 outcome (one-line summary)

`FlowAsTool` body lands in `starter-flow-surfaces` with an
explicit-field builder (D-F3.4), a `Tool` impl that drives a
one-shot `FlowRunner` off `Arc<Engine>` and maps
RunStatus + NodeFailed events into typed `SpiError`, plus
`invoke_with_cancel` / `invoke_with_timeout` helpers; six
unit tests cover every invariant; the stage adds one Cargo
dep (`starter-flow`) on the surfaces crate, dep-tree gates and
the `starter-flow-spi` baseline both stay clean.

## Stage 7 implementation choices made

- **Explicit-fields builder, no convenience constructor.**
  `FlowAsToolBuilder::build` fails fast with
  `FlowAsToolBuildError::MissingField(name)` naming the first
  missing required field. D-F3.4 forbids derive-from-flow-
  revision, so the builder makes the explicit-schema contract
  enforced rather than convention.
- **`SeedAdapter` / `OutputAdapter` are caller-supplied
  closures.** Both are `Arc<dyn Fn(...) + Send + Sync +
  'static>` type aliases. Reason: a flow's input/output JSON
  shape is per-flow and not derivable from the topology in
  Phase 3; the explicit adapter is the matching imperative
  side of D-F3.4's explicit schemas.
- **One-shot `FlowRunner` per `invoke` call.** `FlowAsTool`
  holds `Arc<Engine>` and constructs a fresh `FlowRunner` per
  `invoke`, sharing the engine's `health_handle()` and
  conditionally `run_store()`. Reason: matches Phase 3 SCOPE
  "engine: Arc<Engine>"; lets stage-6 degraded-mode rejection
  fire at `runner.start(...)` instead of requiring duplicated
  health-check logic in the surfaces crate.
- **Cancel-forwarder + event-watcher as `tokio::spawn` tasks
  with explicit `.abort()` on every termination path.** The
  forwarder calls `cancel.cancelled().await; run_cancel.cancel()`;
  the watcher loops `events_rx.recv().await` capturing
  the first `FlowEvent::NodeFailed`. Both are aborted in
  every match arm and on join-error so no per-call task can
  leak. Test 6 (`invoke_does_not_leak_tokio_tasks`) asserts
  this empirically across 16 back-to-back invocations.
- **`FlowEvent::NodeFailed` surfaces as a typed error.** A
  single failing node does not flip `RunStatus` to `Failed`
  (the propagator emits `NodeFailed` and the coordinator
  quiesces normally as `Completed`). The watcher task remembers
  the first `NodeFailed`; on `RunStatus::Completed` the
  surface checks the slot first and returns
  `SpiError::Internal` carrying `"flow run failed: node {n}
  returned {error}"` rather than silently returning the
  output-adapter's fallback (typically `Null`). Reason:
  matches caller expectation that an erroring tool call is
  an `Err`, not an `Ok(Null)`.
- **Tool trait signature is `invoke(input) -> Result<Value>`.**
  The current workspace `Tool` trait in
  `crates/starter-spi/src/tool/kind.rs` carries no
  `Principal` / `Cancel` / `EventSink` args (unlike the
  SCOPE's aspirational "call(args, principal, cancel, sink)"
  shape). `FlowAsTool::invoke_with_cancel` /
  `invoke_with_timeout` are the host-facing escape hatches
  for R13 cancellation; `Tool::invoke` delegates with a fresh
  never-fired `RunCancel`. The hardcoded `"system/Admin"`
  Principal default in stage 5 stays — moving it to a
  `RunSpec::with_principal(...)` extension is deferred to
  stage 8 / a follow-up because the surface has no Principal
  to thread in until the trait evolves.
- **`InMemoryRunStore` for the per-call `FlowRunner::new`
  positional arg.** The runner still requires a non-Phase-3
  `RunStore` for its Phase 2 in-memory accounting. Reason:
  keeps Phase 2 `FlowRunner` API unchanged; the SPI
  `RunStore` (if attached to the engine) is the one that
  actually persists checkpoints.
- **`Cargo.toml` dep additions on `starter-flow-surfaces`.**
  Added `starter-flow`, `async-trait`, `serde_json`,
  `tokio { features = ["macros", "sync", "rt", "time"] }`,
  `tracing`, plus `tokio` dev-dep with `rt-multi-thread`.
  Verified against `workspace_dep_tree_gates`: the
  `no_flow_crate_depends_on_phase3_surfaces` test forbids
  surface → mcp/server/cli only — flow → surface is fine; the
  `starter-flow-spi` baseline test is unaffected (no SPI dep
  edits).

## Stage 7 files touched

- `crates/starter-flow-surfaces/Cargo.toml` — add
  `starter-flow`, `async-trait`, `serde_json`, `tokio`,
  `tracing` runtime deps + `tokio` dev-dep.
- `crates/starter-flow-surfaces/src/lib.rs` — full rewrite:
  `FlowAsTool` struct + builder + `Tool` impl +
  `invoke_with_cancel` / `invoke_with_timeout`; `FlowAsService`
  stays empty (stage 8).
- `crates/starter-flow-surfaces/tests/stage7_flow_as_tool.rs`
  — new file, six tests:
  `builder_rejects_missing_required_fields`,
  `invoke_drives_flow_and_returns_terminal_output`,
  `invoke_surfaces_flow_failure_as_typed_error`,
  `invoke_rejects_while_engine_is_degraded`,
  `invoke_with_cancel_propagates_within_200ms`,
  `invoke_does_not_leak_tokio_tasks`.

## What stage 8 starts with

- **`FlowAsService` body** in
  `crates/starter-flow-surfaces/src/lib.rs` next to
  `FlowAsTool`. Fields per R9 + D-F3.5: engine handle, flow
  topology + terminal slots, `EventSink` subscription, service
  name, lifecycle hooks (`start` → subscribe + spawn, `stop` →
  drain + join), per-event seed adapter, dedup-key resolver
  (D-F3.12: `EventSink::dedup_key()` first, blake3 fallback),
  `FlowEvent::DedupShortCircuit` emission on re-delivery via
  `RunStore::find_by_dedup_key`.
- **`Service` impl from `starter_spi::service::Service`.**
  `start(ctx) -> Result<ServiceHandle>`; the handle's `stop()`
  drains the subscription and joins the inner runner tasks.
- **Tests under `starter-flow-surfaces/tests/`** mirroring
  stage 7's six-test shape: subscribe-on-start, drain-on-stop,
  dedup-short-circuit-emits-event, no-task-leak.
- **Optional stage-8 housekeeping**: land
  `RunSpec::with_principal(...)` so the per-event Principal
  the service derives (host-specific) is threadable into the
  per-event run instead of the stage-5 `"system/Admin"`
  default.

## (Historical) What stage 7 started with

- **`FlowAsTool` body** in
  `crates/starter-flow-surfaces/src/lib.rs`. Fields per R8 +
  D-F3.4 (explicit schemas at construction): `flow_id: FlowId`,
  `engine: Arc<Engine>`, `tool_id: KindId`, `name: String`,
  `description: String`, `input_schema: serde_json::Value`,
  `output_schema: serde_json::Value`.
- **`Tool` impl from `starter_spi::tool::Tool`** that forwards
  `Tool::call(args, principal, cancel, sink)` into the engine by
  constructing a per-call `FlowRunner` (pulling the engine's
  `health_handle()` + `run_store()`), driving the flow, and
  returning the terminal output slot value as the tool's return.
- **Stage 7 inherits the stage-6 plumbing**: the engine's shared
  `HealthHandle` flows through `with_health_handle(...)`; the
  per-run metrics are accessible via `RunHandle::metrics` for
  the span side-channel; `Tool::call`'s `Cancel` parameter
  becomes the per-run `RunCancel`.
- **Span on `flow_as_tool.call`** records `(flow_id, tool_id,
  principal_id_hash, run_id)`. The Phase 2 engine substrate's
  `span = tracing::info_span!("write_slot", ...)` is the existing
  pattern to mirror.
- **Tests cover** happy-path / error mapped to typed `ToolError`
  / cancel-within-200ms / no-tokio-task-leak (span open/close
  balance). All four live in `starter-flow-surfaces/tests/`.

## Stage 7 implementation gotchas

- The stage-5 hardcoded `"system/Admin"` Principal default is
  the right time to revisit: `Tool::call` already carries a
  `Principal`, so `FlowAsTool` should thread that through
  via a `RunSpec::with_principal(...)` extension or
  `FlowRunner::start_for(principal, ...)`.
- `starter-flow-surfaces` already path-deps on `starter-flow`
  (verified by stage-3 dep-tree gates); no Cargo.toml changes
  for the wire-up itself.
- `starter_spi::tool::Tool` is `async_trait`; the `call`
  signature returns `Result<ToolOutput, ToolError>`. Mapping
  `RunStatus::Failed(error)` to a typed `ToolError` variant is
  the spot to look at next.
- The `Cancel`-to-`RunCancel` plumbing already exists at
  `RunHandle::cancel`; `FlowAsTool::call` registers a watcher
  task that calls `handle.cancel.cancel()` when the incoming
  `Tool::call`'s cancel fires.

## Phase-3 follow-up notes (not in scope for this job)

- 1M-tick long-uptime soak as a nightly CI job (Q6).
- `RunOpts.checkpoint_backoff` if a consumer surfaces a real
  need (Q8); stage 6 hardcodes
  `CHECKPOINT_BACKOFF_MS = [50, 100, 200, 400, 800]`.
- `FlowEvent::HealthChanged` engine-level event once a
  Phase-7-owned engine-level event bus exists (Q7).
- Cancellation-aware retry loop (`select!` over
  `tokio::time::sleep` + `cancel.cancelled()`) so an outage that
  coincides with a fired cancel exits sooner than the longest
  backoff (800 ms). Deferred — SCOPE doesn't require it; cancel
  during retry already exits cleanly at the next `sub.next()`
  await.
- `starter-store-postgres` `flow` feature mirror (D-F3.3 revisit
  trigger).
- Add a `subject` indexed column to `sessions` if a hot
  per-principal listing path surfaces.
- Pre-existing workspace fmt drift in theme/ui-theme/notes
  crates (out-of-scope).
- Pre-existing `starter-grpc` clippy failure under
  `--workspace --all-targets` (needs `--features testing`).
- A `RunSpec::with_principal(...)` / `FlowRunner::start_for
  (principal, ...)` extension to replace the stage-5 hardcoded
  "system/Admin" Principal default — best landed in stage 7/8
  with the surfaces (now imminent).
- `FlowRunnerConfig.event_buffer` field is now dead code (stage
  6 reads `RunOpts.event_broadcast_capacity` instead). Left in
  place to avoid a breaking-config churn on Phase 2 callers;
  consider removing in a follow-up cleanup once Phase 3 ships.
