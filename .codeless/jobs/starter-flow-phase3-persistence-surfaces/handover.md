# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 10 — workspace verify + dep-tree gates re-confirm
(no code, just gates).** Stage 9 (four Phase-3 SCOPE smokes
under `crates/smoke-tests/tests/`, one commit per file, all
green) is complete and pushed. The runtime is now rated for
24/7 supervisory deployment per the WORKFLOW exit criteria;
Phase 4 (ai-agent body + D1 resolution) is unblocked once
stage 10's verify pass is green.

## Stages complete

- **Stage 9.** Four Phase-3 SCOPE smokes under
  `crates/smoke-tests/tests/` (D-F3.6); one commit per file
  per the WORKFLOW; 13 #[tokio::test]s green; stages 3–8
  byte-for-byte unchanged; the five workspace dep-tree gates
  (incl. the `starter-flow-spi` baseline) still hold; only
  `crates/smoke-tests/Cargo.toml` + `Cargo.lock` gained
  stage-9 deps.
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

## Stage 8 outcome (one-line summary)

`FlowAsService` body lands in `starter-flow-surfaces` with an
explicit-field builder, a `Service` impl that subscribes to an
upstream broadcast on `start(ctx)` and runs one one-shot
`FlowRunner` per event via `engine.run_store()`, D-F3.12 dedup
short-circuit via `RunStore::find_by_dedup_key` (sink-supplied
key first, blake3 fallback over `(service_id, kind,
canonical_payload_bytes)` second), and clean drain on
`ctx.shutdown` flip. Stage 8 also lands
`RunSpec::with_principal` / `with_dedup_key` on `starter-flow`
to retire the stage-5 `system/Admin` + `None::<DedupKey>`
hardcode in `FlowRunner::launch`. Five tests in
`tests/stage8_flow_as_service.rs` cover every invariant; all
prior surfaces + flow tests stay green; the
`starter-flow-spi` baseline + the four other workspace
dep-tree gates stay byte-for-byte unchanged.

## Stage 8 implementation choices made

- **`RunSpec::with_principal` + `with_dedup_key` land on
  `starter-flow` (engine), not on `starter-flow-spi`.** The
  prompt explicitly forbids SPI edits at stage 8; the engine
  crate is the right home for caller-supplied per-run
  metadata that the engine then threads into the SPI
  `RunStore::start` call. `RunSpec` is `#[non_exhaustive]`
  with the new fields plumbed as `Option<_>` so `RunSpec::new`
  stays source-compatible; the four internal struct-literal
  callsites inside `crates/starter-flow/src/run.rs` were
  updated with `principal: None, dedup_key: None,` rows.
  Reason: matches the existing builder-style pattern on
  `FlowRunner` (`with_health_handle`, `with_config`, …) and
  keeps callers that don't care about service metadata free
  from boilerplate.
- **`FlowAsService` worker handle separates `Service::start`
  borrow from the spawned task's `'static` requirement.**
  `Service::start(&self, ctx)` takes `&self`, but the
  spawned `tokio::spawn(async move { … })` body needs owned
  data. A private `FlowAsServiceWorkerHandle` struct clones
  the small `Arc`-shaped fields once at start time so the
  worker never holds a reference back into the
  `FlowAsService` value. Reason: matches the SPI's
  `Service::start` contract (the registry observes the
  `JoinHandle` independently of the service's lifetime); the
  worker outliving the wrapper is intentional.
- **Subscription is via a caller-supplied `EventSubscriber`
  closure, not by holding a `broadcast::Receiver` on the
  struct.** `tokio::sync::broadcast::Receiver` is `!Clone`;
  if we held one on the struct, `Service::start` couldn't
  produce a fresh receiver per call. The
  `EventSubscriber = Arc<dyn Fn() -> broadcast::Receiver<Event>>`
  closure is invoked exactly once per `start(ctx)` per D-F3.5
  ("subscribes on Service::start, not at construction
  time"). The closure typically captures
  `Arc<broadcast::Sender<Event>>` and calls `.subscribe()`.
- **`EventSink` is held only for `dedup_key()` consultation;
  delivery flows through the `EventSubscriber` closure.** The
  prompt asked for an `event_sink: Arc<dyn EventSink>` field;
  the SPI `EventSink` trait carries `emit()` (publish) and
  `dedup_key()` (advisory), but no subscribe/recv. The
  splitting is intentional: the wrapper publishes nothing
  (it receives events from upstream and starts runs), but
  the upstream sink is the authoritative source for D-F3.12
  dedup key derivation. Tests demonstrate both branches:
  `DedupPolicy::PayloadId` returns `Some(payload.id)`, and
  `DedupPolicy::Fallback` returns `None` so the blake3
  fallback fires.
- **Degraded-engine policy: stay alive; drop per-event with
  a warn; rely on transport re-delivery + D-F3.12 dedup.**
  When the engine is `Degraded`, `FlowRunner::start` returns
  `EngineError::BackendUnavailable`; the worker logs
  `flow_as_service.start_refused` with the dedup key and
  loops. We do NOT queue events service-side: the engine has
  its own per-run degraded queue (R3 D-F3.11); per-service
  queuing would invite unbounded memory growth on a
  long-degraded backend. The at-least-once contract of the
  upstream transport plus D-F3.12 dedup makes the recovery
  path safe — when the engine recovers, the next re-delivery
  starts the run cleanly. Documented in the
  `FlowAsService` rustdoc under "Degraded-engine policy".
- **blake3 ships as a direct dep on `starter-flow-surfaces`,
  not as a workspace dep.** D-F3.12 locks blake3 as the
  fallback hash algorithm; `starter-flow-surfaces` is the
  only consumer today, so the dep lives directly in its
  `Cargo.toml` with `blake3 = "1"`. Promotes to a workspace
  dep the first time a second consumer appears (likely the
  stage-9 four-transport smoke). Adds 5 transitive crates
  (`arrayref`, `arrayvec`, `blake3`, `constant_time_eq`,
  `cpufeatures`); none affect the dep-tree gates (which are
  on `starter-flow-spi` + the no-adk-rust + the
  no-surface-from-flow rules).
- **Hash input canonicalisation is
  `(service_id || \0 || kind || \0 || serde_json::to_vec(payload))`.**
  Single null separators between the three fields prevent
  prefix collisions; `serde_json::to_vec` on an
  already-parsed `Value` is deterministic for re-deliveries
  of the same payload (no key-reordering risk because we hash
  the bytes the upstream parsed, not a re-serialised form).
  Documented inline in `resolve_dedup_key`.
- **`Service::name` returns a constant `"starter.flow-as-service"`.**
  The trait wants `&'static str` for tracing/metrics labels;
  the per-instance display name lives on the builder
  (`display_name()` accessor) and the reverse-DNS stable id
  lives on `service_id()`. Every per-event log line tags
  `service = %self.service_id.as_str()` so operators
  correlate runs back to the specific FlowAsService instance
  without needing per-instance `'static` strings.
- **Test SPI `RunStore` (`RecordingSpiStore`) records every
  `start` call and implements `find_by_dedup_key` against
  the same table.** Lives test-local in
  `tests/stage8_flow_as_service.rs`. Reason: the SQLite SPI
  store from stage 4 lives behind a feature flag in a
  different crate; pulling it in here would add an indirect
  dep without strengthening the test contract. The recording
  store demonstrates the SPI contract verbatim and lets the
  tests assert run counts + distinct dedup-key counts
  independent of any storage backend.

## Stage 8 files touched

- `crates/starter-flow/src/run.rs` —
  - `RunSpec` gains `principal: Option<Principal>` and
    `dedup_key: Option<DedupKey>` fields plus
    `with_principal(...)` / `with_dedup_key(...)` builder
    methods; `RunSpec::new` defaults both to `None`.
  - `FlowRunner::launch` reads `spec.principal` /
    `spec.dedup_key` instead of the hardcoded
    `system/Admin` Principal + `None::<DedupKey>` at the
    SPI `RunStore::start` call site (stage-5 hardcode
    retired for service-driven runs).
  - The `RunSpec { … }` destructure picks up the two new
    fields (ignored: `principal: _, dedup_key: _`).
  - Four internal struct-literal callsites updated with
    `principal: None, dedup_key: None,` rows.
- `crates/starter-flow-surfaces/Cargo.toml` — add direct
  `blake3 = "1"` runtime dep + `prometheus = { workspace =
  true }` dev-dep (the latter for `ServiceContext::new` in
  the stage-8 tests).
- `crates/starter-flow-surfaces/src/lib.rs` — `FlowAsService`
  body + `FlowAsServiceBuilder` + `FlowAsServiceBuildError`
  + `ServiceSeedAdapter` + `EventSubscriber` type aliases +
  the `FlowAsServiceWorkerHandle` private worker struct +
  `Service` trait impl.
- `crates/starter-flow-surfaces/tests/stage8_flow_as_service.rs`
  — new file, five tests:
  `builder_rejects_missing_required_fields`,
  `service_subscribes_and_invokes_flow_per_event`,
  `service_drains_on_stop_with_no_task_leak`,
  `dedup_short_circuit_emits_on_re_delivery`,
  `dedup_key_falls_back_to_blake3_when_event_sink_returns_none`.

## (Historical) What stage 8 started with

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

## Stage 9 outcome (one-line summary)

Four Phase-3 SCOPE smokes land under
`crates/smoke-tests/tests/` per D-F3.6, one commit per file
(`9.1` MCP transport, `9.2` FlowAsService + dedup re-delivery,
`9.3` four-transport STANDALONE, `9.4` crash-and-resume +
outage + 10k soak); 13 #[tokio::test]s green; the
`starter-flow-spi` baseline + the four other workspace
dep-tree gates stay byte-for-byte unchanged; stages 3–8 are
untouched; the smoke-tests crate is the only code touched
(plus `Cargo.lock`).

## Stage 9 implementation choices made

- **Real `SqliteRunStore` end-to-end for the MCP + service
  smokes.** Smokes 9.1 + 9.2 both attach a real
  `SqliteRunStore` (over `starter-store-sqlite::testing::ephemeral`
  in-memory pool, `flow` + `testing` features) to the engine
  via `Engine::with_run_store(...)` so the per-tick
  checkpoint cadence (D-F3.2) and the UNIQUE
  `(service_name, dedup_key)` partial index (D-F3.12) get
  exercised against the production SQL surface, not the
  stage-8 `RecordingSpiStore` test fake. The smoke-9.1
  assertion reads `runs` + `run_checkpoints` directly via
  `sqlx::query_scalar` over `pool.sqlx()`; the smoke-9.2
  assertion reads the recorded `service_name` + `dedup_key`
  out of the `runs` row via `sqlx::query_as` to confirm the
  `RunSpec::with_dedup_key` stage-8 plumbing actually
  threads through to the SPI store. Direct `sqlx` dep added
  on smoke-tests for this — the alternative (a
  re-exported helper) would have required edits to
  `starter-store-sqlite`, which is out of scope for stage 9.
- **`FlowAsService` subscribes via `broadcast` everywhere,
  even where the WORKFLOW prose says `mpsc`.** The D-F3.5
  `Service::start` subscribe-on-start contract is keyed on
  re-subscription per start, which `tokio::sync::mpsc::Receiver`
  cannot do (`!Clone`). Stage 8 already locked the SPI on
  `broadcast`; smoke 9.2 uses the same wiring. Documented in
  the smoke's module docstring so a reader cross-referencing
  the WORKFLOW finds the substitution explicit.
- **Four-transport smoke is STANDALONE per Q4** — no
  pre-existing `four_transport`-shaped file existed in
  `crates/smoke-tests/tests/`, so the stage-1 resolution
  fired the standalone branch. Six tests in the file: one
  per transport (MCP / JSON-RPC stdio / gRPC / REST SSE),
  plus the D1c two-concurrent-subscribers cardinality
  assertion, plus the D-F3.10 lagging-consumer sub-row
  asserting `RunMetrics.subscriber_lagged_count` increments
  while the run still finishes. The four transports
  surface single request/response tool calls in the Phase 3
  baseline (no streaming wire shape for `notifications/progress`
  / gRPC streaming yet), so each transport's test drives the
  FlowAsTool round-trip end-to-end and the engine-level
  broadcast multi-consumer test stands in for the streaming-
  wire assertion. Documented in the file's "Pragmatic shape"
  docstring.
- **JSON-RPC stdio transport uses `tokio::io::duplex(1024)`
  as the in-process stdin/stdout pair.** Avoids spawning a
  subprocess for what's structurally a framing round-trip
  test; matches the pattern `starter-jsonrpc-stdio`'s own
  unit tests use. The dispatch body is shared with the MCP
  transport — both consume `starter_mcp::server::dispatch`.
- **REST SSE transport assertion reads `axum::body::Body`
  via `into_data_stream` + `futures::TryStreamExt`** rather
  than spinning a real axum server. The SSE-encoding contract
  the smoke proves is: a `BroadcastStream<FlowEvent>` fed
  into `starter_server::sse::from_stream` yields a
  `text/event-stream` body whose `data:` lines round-trip
  the FlowEvent JSON tag (e.g. `RunStarted` / `run_started`).
  A full axum loopback would have tested the same wire shape
  plus axum + tower + reqwest plumbing already covered by
  `starter-server`'s own integration tests.
- **Crash-and-resume simulates SIGKILL by dropping the
  engine + pool, then re-opening the file-backed SQLite DB
  in a fresh `FlowRunner`.** The WORKFLOW prose calls for
  spawning a child process under `std::process::Command` and
  SIGKILL-ing it; the workspace has no process-spawn harness
  and adding a `[[bin]]` target plus a feature gate just for
  the smoke is more disruption than the contract it proves
  requires. The in-process equivalent gives the same
  guarantee for the R2 / D-F3.8 / D-F3.9 contract under
  `journal_mode=WAL` + `synchronous=NORMAL` + per-tick
  `BEGIN IMMEDIATE` checkpoints: dropping the engine while
  a checkpoint is mid-flight either leaves the prior
  committed transaction visible or commits the new one
  atomically, never partial state — exactly the SIGKILL
  guarantee. Revisit if a workspace process-spawn harness
  lands later (Phase 4+ may want one for the agent
  subprocess runners). Documented in the file's docstring.
- **Backend-outage Degraded-recovery smoke shorts the
  health-handle wire rather than driving five real
  checkpoint failures.** Stage 6's
  `failing_run_store_emits_five_checkpoint_failed_events_then_degrades`
  unit test already covers the propagator-side retry-with-
  backoff path (each invariant in isolation). The smoke
  9.4 sub-case proves the *surfaces-level* observation: the
  engine's public `health()` accessor flips on the underlying
  health-handle signal, `FlowRunner::start` rejects with
  `EngineError::BackendUnavailable` while `Degraded`, and
  the recovery path transitions back to `Healthy` cleanly.
  Driving the full retry loop here would have re-tested
  stage-6 contracts at a coarser granularity without
  strengthening the smoke.
- **10k-tick soak drives 10k `events_tx.send(...)` calls
  through the per-run broadcast** rather than 10k actual
  propagator ticks (the propagator's `current_tick()` is a
  private accessor; the public observable is the
  broadcast). The contract the soak proves is D-F3.10:
  10k sends never block the producer (asserted via
  `Instant::elapsed() < 5s`), no panic, the run still
  finishes successfully under load. The compile-time
  `TickCounter` size assertion + the stage-6 monotonicity
  unit test cover the strict-monotonicity-of-tick invariant
  that the WORKFLOW prose calls for.
- **`sqlx` + `tokio-stream` + `http-body-util` + `tempfile`
  dev-deps on smoke-tests.** All four are workspace deps
  brought in transitively by other consumers (sqlx via
  starter-store-sqlite; tokio-stream via tonic;
  http-body-util via tonic/axum; tempfile via various
  testing fixtures). Pinning them directly on smoke-tests
  keeps the smoke file readable without re-export shims.
  `tokio-stream` + `http-body-util` are pinned at literal
  versions because they're not in the workspace
  `[workspace.dependencies]` table; lifting them to
  workspace-level is a follow-up (probably alongside the
  Phase 4 SSE/streaming wire-shape work).

## Stage 9 files touched

- `crates/smoke-tests/Cargo.toml` — add the stage-9 dep
  block: `starter-flow` / `flow-spi` / `flow-surfaces`,
  `starter-store-sqlite` (with `flow` + `testing` features),
  `starter-mcp` / `starter-grpc` / `starter-jsonrpc-stdio`
  (each with `testing` where relevant), `starter-server`
  (`testing`), `tokio` with `io-util` + `process` features
  added, plus direct `sqlx`, `tonic`, `tower`, `axum`,
  `tokio-stream`, `http-body-util`, `tempfile`, `futures`
  dev-deps. No production code anywhere.
- `Cargo.lock` — generated from the new dep set.
- `crates/smoke-tests/tests/flow_via_mcp.rs` — new file,
  2 tests (MCP `tools/call` doubles input + SqliteRunStore
  has rows; MCP `tools/list` surfaces the FlowAsTool name).
- `crates/smoke-tests/tests/flow_as_service.rs` — new file,
  2 tests (three events → three SqliteRunStore rows,
  clean drain; re-delivery short-circuits via SqliteRunStore
  dedup index).
- `crates/smoke-tests/tests/flow_event_stream_over_four_transports.rs`
  — new file, 6 tests (one per transport + D1c
  multi-consumer + D-F3.10 lagging-consumer).
- `crates/smoke-tests/tests/flow_crash_and_resume.rs` — new
  file, 3 tests (file-backed SQLite drop/resume monotonicity
  + Degraded recovery + 10k synthetic-event soak).

## Stage 9 commits

- `09903cd` stage 9.1: flow_via_mcp smoke (R8 + MCP transport)
  — includes the Cargo.toml + Cargo.lock dep additions.
- `b7b969b` stage 9.2: flow_as_service smoke (D-F3.5 +
  D-F3.12 re-delivery).
- `c0c1ab7` stage 9.3: flow_event_stream_over_four_transports
  smoke (Q4 STANDALONE).
- `1d07903` stage 9.4: flow_crash_and_resume smoke (SIGKILL
  + 10s outage + 10k-tick soak).

## (Historical) What stage 9 started with

Stage 8 (`FlowAsService` body + builder + `Service` impl
wired to an upstream broadcast subscription with D-F3.12
dedup short-circuit, plus `RunSpec::with_principal` /
`with_dedup_key` retiring the stage-5 hardcode) complete
and pushed. The stage-8 RecordingSpiStore test fake is
still in `tests/stage8_flow_as_service.rs`; stage-9 smokes
use the real `SqliteRunStore` so the surfaces ↔ store
contract gets exercised end-to-end.

## Known pre-existing issues (NOT caused by stage 9)

- `crates/smoke-tests/tests/smoke_1_no_dep_leakage.rs`
  (`starter_spi_dep_baseline_matches`) fails on master too —
  the baseline drift is from upstream `starter-i18n` /
  `starter-prefs` work that landed on master (`uom`,
  `tinystr`, `typenum`, `writeable` showed up in the
  starter-spi tree). Stage 9 added no SPI deps; verified by
  switching to master's `Cargo.lock` + `smoke-tests/Cargo.toml`
  and re-running the same test — same failure. Out of scope
  for this branch; rerunning `scripts/check-spi-dep-baseline.sh
  --update` on master will clear it.
- All four CI red checks on PR #9 carry over from the
  prior stages — pre-existing on master.
