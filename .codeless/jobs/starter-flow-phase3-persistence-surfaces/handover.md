# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 6 — durability hardening.** Stage 5 (engine wiring +
resume-from-checkpoint + R2 chokepoint test) is complete and
pushed.

## Stages complete

- **Stage 1.** D-F3.1..D-F3.12 + Q1..Q8 locked; Q4 → STANDALONE.
- **Stage 2.** REVIEW gate passed.
- **Stage 3.** SPI trait fleshout + baseline regeneration.
- **Stage 4.** Three SQLite store impls behind default-off `flow`
  feature; 8 integration tests green; workspace gates clean.
- **Stage 5.** Engine wiring: `Engine::with_run_store(...)`
  builder, per-tick checkpoint cadence in propagator, resume-
  from-checkpoint on `FlowRunner::resume(...)`, R2 chokepoint
  integrity test (`stage5_resume_chokepoint.rs`) green. All
  three Phase 2 smokes still green.

## Stage 5 outcome (one-line summary)

`FlowRunner::with_spi_run_store(Arc<dyn spi::RunStore>)` +
`Engine::with_run_store(...)` plus a new `propagator::
CheckpointHook` thread per-tick `RunStore::checkpoint(run, seq,
state, &writes)` calls into the SPI store; `FlowRunner::resume
(spec, input, run_id)` loads the latest checkpoint and replays
its writes through the single `GraphStore::write_slot`
chokepoint (R2 unchanged), then spawns a fresh propagator with
`initial_seq = checkpoint.seq` so the first post-resume
checkpoint carries `seq = checkpoint.seq + 1`.

## Stage 5 implementation choices made

- **`Engine::with_run_store(...)` is a passive holder; the wiring
  lives on `FlowRunner`**. Reason: the `Engine` state machine
  (R12) does not spawn runs — `FlowRunner` does. Putting the
  active hook on `Engine` would require either a major API shift
  (`Engine::start_run(...)` etc.) or threading the engine into
  the FlowRunner. The handover called for both names; the
  SCOPE-named `Engine::with_run_store` is preserved as a builder
  + `Engine::run_store()` accessor that surface adapters
  (stage 7/8 `FlowAsTool`/`FlowAsService`) will pull from when
  constructing per-call FlowRunners.
- **Two `RunStore` traits coexist**. The Phase-2 in-memory
  `crate::run::RunStore` (record/get/len) is kept verbatim so
  existing callers don't break; the Phase-3 SPI
  `starter_flow_spi::flow::RunStore` (start/checkpoint/load/
  finish/list_open/find_by_dedup_key) is attached additively via
  `FlowRunner::with_spi_run_store(...)`. When unattached, the
  runner behaves exactly as Phase 2 (matches SCOPE: "when no
  RunStore is attached, the engine behaves exactly as it does
  today").
- **Per-tick batch lives only when a hook is attached**. The
  propagator skips the `tick_writes: Vec<(SlotRef, SlotValue)>`
  push when `checkpoint.is_none()` so the no-store path
  allocates nothing. Writes are mirrored into the batch only on
  `write_slot` success, so a failed fan-out doesn't poison the
  checkpoint.
- **One checkpoint per tick, not per write**. Per D-F3.2: the
  propagator's "tick" = one event-handler iteration; all fan-out
  + node-output writes during that iteration land in one
  `RunStore::checkpoint(...)` call. Seq is monotonic per-run via
  the existing propagator `hops: u64` counter plus the
  `initial_seq` offset for resume.
- **Stage 5 logs checkpoint failures and continues**. Per the
  WORKFLOW per-stage table, retry-with-backoff +
  `EngineHealth::Degraded` is Stage 6's job; stage 5 emits a
  `warn!` and lets the run continue. The
  `FlowEvent::CheckpointFailed { attempt }` event is reserved
  for stage 6 (it requires the retry loop to fill `attempt`).
- **SPI `start`/`finish` lifecycle wired**. `FlowRunner::launch`
  calls `spi.start(run_id, revision, opts, Principal, None)`
  before spawning the coordinator (fresh runs only; resumed
  runs skip to avoid PK collision on `runs.run_id`). The
  coordinator calls `spi.finish(run_id, outcome)` after the
  terminal status is set so SCOPE D-F3.9 "final-checkpoint
  preserved" is honoured.
- **Default Principal is a hardcoded "system/Admin"** at the
  `FlowRunner` boundary. Reason: `RunSpec` is `#[non_exhaustive]`
  and bumping it with a `principal` field is best done when the
  surfaces (stage 7/8) ship — they're the natural Principal
  source. The placeholder is unambiguous in logs; stage 7/8
  replaces it via a `RunSpec::with_principal(...)` extension
  or a richer `FlowRunner::start_for(principal, ...)` overload.
- **New direct dep: `starter-spi` on `starter-flow`**. Reason:
  constructing a `Principal { subject, role, scopes, extra }`
  needs the `Role` enum which `starter-flow-spi` re-exports for
  `Principal` only. `starter-flow-spi` already pulls
  `starter-spi` transitively so the engine's dep tree gains
  nothing new (the `workspace_dep_tree_gates` test stays green;
  the `starter-flow-spi` baseline is unchanged). Added with
  `serde_json` for the `Principal.extra` field.
- **`RunSpec::new(...)` constructor added**. The stage-5 test
  lives under `crates/starter-flow/tests/` (external crate
  boundary) and cannot use the struct-expression form because
  of `#[non_exhaustive]`. The constructor is also the right
  ergonomics for future surfaces.
- **`propagator::spawn_with_checkpoint` + `drive_with_checkpoint`
  are new entry points; `spawn`/`drive` delegate**. Reason: the
  existing `spawn`/`drive` signatures are public and called by
  three external sites (`starter-flow/tests/
  smoke_one_write_chokepoint.rs`, `starter-flow-nodes/tests/
  transform_node_failed.rs`, the engine internal coordinator).
  A breaking change would mean retroactively touching those
  files; the delegate keeps them green and the new APIs
  surface the hook explicitly.

## Stage 5 files touched

- `crates/starter-flow/Cargo.toml` — added `starter-spi` +
  `serde_json` direct deps.
- `crates/starter-flow/src/engine.rs` — `Engine::with_run_store`
  + `Engine::run_store` + `run_store: Option<Arc<dyn spi::
  RunStore>>` field.
- `crates/starter-flow/src/propagator.rs` — `CheckpointHook`
  struct + `spawn_with_checkpoint` / `drive_with_checkpoint`
  entry points + per-tick batch collection + one
  `RunStore::checkpoint` call per non-empty tick.
- `crates/starter-flow/src/run.rs` — `FlowRunner::
  with_spi_run_store` / `with_run_opts` / `spi_run_store`
  accessors; new `FlowRunner::resume(spec, input, run_id)`
  method; new `FlowRunnerError` enum; `RunSpec::new(...)`
  constructor; coordinator threads spi store through to
  `propagator::spawn_with_checkpoint` and calls
  `spi.start(...)` / `spi.finish(...)` on the lifecycle.
- `crates/starter-flow/tests/stage5_resume_chokepoint.rs` —
  new test asserting (a) checkpoint history accumulates during
  a fresh run, (b) resume returns Some when a checkpoint
  exists, (c) replay writes go through the `GraphStore::
  write_slot` chokepoint (counted via a wrapper store), and
  (d) every replayed slot lands with the checkpoint's value.

## Known pre-existing issues (NOT caused by stage 3/4/5)

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
  `starter-flow-nodes` (Cargo wants the workspace root to
  declare `default-features` for the warning to land
  cleanly). Out of scope.

## Branch + commits

- Branch: `codeless/starter-flow-phase3-persistence-surfaces`.
- Stage 1 commit: `8407d6e` — decision lock.
- Stage 3 commit: `44f19f5` — SPI fleshout + baseline regeneration.
- Stage 4 commit: `8a60ddb` — SQLite store impls.
- Stage 5 commit: see latest log — engine wiring + resume + R2
  chokepoint test.
- Pushed to origin.

## What stage 6 starts with

- **Retry-with-backoff** on `RunStore::checkpoint` errors per
  D-F3.11 (50→100→200→400→800ms, 5 attempts, then
  `EngineHealth::Degraded`). Replaces the stage-5
  `tracing::warn!("checkpoint failed; log and continue")`
  callsite in `propagator::drive_with_checkpoint`.
- **`EngineHealth { Healthy, Degraded }` accessor** on
  `Engine` (the type already exists in
  `starter_flow_spi::flow`); plus `Engine::health() ->
  EngineHealth` reading an `AtomicU8` so the lookup is
  lock-free.
- **Per-run in-memory checkpoint queue** under
  `RunOpts.degraded_queue_capacity` (default 1024, evict-
  oldest). When in `Degraded` the engine queues batches and
  drains in `(run_id, seq)` order on the next successful
  checkpoint write.
- **`Engine::start` returns `EngineError::BackendUnavailable`**
  while `Degraded`. The Phase-2 `Engine::start()` lifecycle
  method (Starting → Running) is unchanged; this is the
  per-run `FlowRunner::start(...)` path that gains the
  rejection. Probably surfaces as a new `Result<RunHandle,
  EngineError>` return type — that's a public API change so
  worth confirming the shape before commit.
- **Per-run broadcast capacity hook** wired into
  `FlowRunner::launch` using
  `RunOpts.event_broadcast_capacity` (currently the runner
  uses the fixed `FlowRunnerConfig.event_buffer` 256).
- **Engine's own `Lagged`-watching subscriber** on every
  per-run broadcast that increments
  `RunMetrics.subscriber_lagged_count`.
- **Monotonic `u64` tick-counter assertion** + the
  `const _: () = assert!(std::mem::size_of::<TickCounter>()
  == 8)` compile-time check. The current propagator counter
  is a plain `u64` local; promoting it to a named
  `TickCounter` newtype is the least-invasive way to satisfy
  the assertion.

## Stage 6 implementation gotchas

- The `RunOpts` shape is `#[non_exhaustive]` and the
  `degraded_queue_capacity` + `event_broadcast_capacity`
  fields already exist (Stage 3 landed them); stage 6 just
  reads them rather than adding them.
- The Phase 3 `FlowEvent::CheckpointFailed { attempt }`
  variant already exists in the SPI (Stage 3 landed it); stage
  6 just emits it from the retry loop. Same for
  `FlowEvent::DedupShortCircuit` — stage 6 doesn't emit it
  (stage 8 / `FlowAsService` does); stage 5 already verified
  the variant doesn't accidentally short-circuit
  `BackendUnavailable`.
- The `EngineHealth` type lives in `starter_flow_spi::flow`;
  the `Engine::health()` accessor in `starter-flow` just
  returns the SPI type. Same for `EngineError`.
- Adding a `tick_counter` newtype touches the propagator's
  `hops: u64` local. The `CheckpointHook.initial_seq` field
  added in stage 5 stays — they compose: `seq =
  initial_seq + tick_counter`.

## Phase-3 follow-up notes (not in scope for this job)

- 1M-tick long-uptime soak as a nightly CI job (Q6).
- `RunOpts.checkpoint_backoff` if a consumer surfaces a real
  need (Q8).
- `FlowEvent::HealthChanged` engine-level event once a
  Phase-7-owned engine-level event bus exists (Q7).
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
  with the surfaces.


