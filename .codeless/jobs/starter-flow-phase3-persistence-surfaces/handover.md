# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 5 — engine wiring: `Engine::with_run_store(...)` builder hook,
per-tick checkpoint cadence, resume-from-checkpoint on
`Engine::start(known_run_id)`.** Stage 4 (SQLite store impls) is
complete and pushed.

## Stages complete

- **Stage 1.** D-F3.1..D-F3.12 + Q1..Q8 locked; Q4 → STANDALONE.
- **Stage 2.** REVIEW gate passed.
- **Stage 3.** SPI trait fleshout + baseline regeneration.
- **Stage 4.** Three SQLite store impls behind default-off `flow`
  feature; 8 integration tests green; workspace gates clean.

## Stage 4 outcome (one-line summary)

`SqliteFlowStore` + `SqliteRunStore` + `SqliteSessionStore` land in
`crates/starter-store-sqlite/src/flow/` behind a new default-off
`flow` cargo feature (D-F3.3, D-F3.7). Five-table schema in
`migrations/flow/0001_init.sql` (`flow_revisions`, `flow_heads`,
`runs` + `UNIQUE (service_name, dedup_key)` partial index,
`run_checkpoints` keyed on `(run_id, seq)`, `sessions`). Pool's
`connect()` extended with an `after_connect` hook applying the
four pragmas workspace-wide (`journal_mode=WAL,
synchronous=NORMAL, busy_timeout=5000, foreign_keys=ON`).

## Stage 4 implementation choices made

- **Pool pragmas are workspace-wide, not flow-feature-gated**.
  Reason: connection pragmas are per-connection in SQLite — the
  pool either applies them on every checkout or it doesn't. Each
  pragma is a safe default for every existing consumer (`:memory:`
  silently ignores `journal_mode=WAL`; `foreign_keys=ON` matches
  what every other store crate's tests already expect). Stage-4
  test `wal_pragmas_applied_on_file_backed_pool` locks this on
  a file-backed pool where journal_mode actually takes effect.
- **`BEGIN DEFERRED` (sqlx default), not `BEGIN IMMEDIATE`**.
  D-F3.8 names atomicity, not lock-upgrade-deadlock avoidance —
  the engine has a single-writer-per-run invariant so the
  upgrade-from-SHARED race doesn't apply. If a later stage
  surfaces multi-writer contention, `Pool::begin_with` in sqlx
  0.8 supports the upgrade additively.
- **Retention loaded outside the checkpoint tx**. `RunOpts` is
  immutable for a run's lifetime (set at `start`); reading it
  inside the tx would only add lock contention.
- **`INSERT OR IGNORE` on `flow_revisions.put`**. Revisions are
  immutable per SCOPE "Decisions made"; a re-put of the same
  `(flow_id, revision_id)` pair is a no-op. Mismatched body on
  re-put is a caller bug the engine layer guards (stage 5).
- **`sessions.list(principal)` scans + filters by `subject`**.
  Sessions are small-cardinality per principal; an indexed
  `subject` column lands additively when a hot listing path
  surfaces.
- **Status mirroring**. `runs.status` mirrors the engine-typed
  `RunState` written into each checkpoint so `list_open` is a
  one-shot indexed lookup against `finished_at IS NULL`.
- **Wildcard match arms on `RunState` / `RunOutcome`**. Both are
  `#[non_exhaustive]` in the SPI; unknown variants map to safe
  defaults (`"running"` / `"failed"` respectively) rather than a
  `Backend` error that would mask the actual checkpoint write.
- **Migrator export**. `starter_store_sqlite::flow::FLOW_MIGRATOR`
  (`static sqlx::migrate::Migrator`) +
  `FLOW_MIGRATION_SOURCE` const so consumers can do
  `migrate(&pool).with_source(FLOW_MIGRATION_SOURCE)` without
  knowing the table name.

## Known pre-existing issues (NOT caused by stage 3 or 4)

- `cargo clippy --workspace --all-targets -- -D warnings` fails
  on master too with `error[E0432]: unresolved import
  'starter_grpc::testing'` in
  `crates/starter-grpc/tests/tools_service.rs` — needs
  `--features testing` which workspace clippy doesn't add.
- `cargo fmt --check` reports pre-existing drift in:
  `crates/starter-spi/src/ui/theme/mod.rs`,
  `crates/starter-ui-theme/src/{lib,routes}.rs`,
  `examples/notes/src/server.rs`.

## Branch + commits

- Branch: `codeless/starter-flow-phase3-persistence-surfaces`.
- Stage 1 commit: `8407d6e` — decision lock.
- Stage 3 commit: `44f19f5` — SPI fleshout + baseline regeneration.
- Stage 4 commit: see latest log — SQLite store impls.
- Pushed to origin.

## What stage 5 starts with

- `Engine::with_run_store(Arc<dyn RunStore>) -> Self` builder
  hook on the engine crate (additive — non-store consumers keep
  the no-store ctor).
- Per-tick checkpoint cadence (D-F3.2): the propagator collects
  the per-tick slot-write batch and calls
  `RunStore::checkpoint(run_id, seq, state, &writes)` once at
  the end of each propagation tick. `seq` is the propagator's
  existing monotonic tick counter (Q2).
- Resume-from-checkpoint: `Engine::start(known_run_id)` loads
  `RunStore::load(run_id)`, replays `RunCheckpoint::writes`
  through `GraphStore::write_slot` (R2 unchanged — the resume
  path is **not** a second writer), then hands off to the
  propagator.
- Checkpoint-failure retry: 5 attempts with exponential backoff
  per D-F3.11; on the 5th failure the engine transitions to
  `EngineHealth::Degraded` and starts buffering checkpoint
  batches into a per-run in-memory queue
  (`RunOpts::degraded_queue_capacity`, evict-oldest on overflow).
  `Engine::start` rejects new runs with
  `EngineError::BackendUnavailable` while degraded.
- The two-`EngineError` reconciliation question lands here.
  Recommendation: keep them separate — engine-internal
  `IllegalTransition` is a propagator-state-machine concern,
  SPI-level `BackendUnavailable + Flow(#[from] FlowError)` is
  the public surface. Wire engine-internal failures into the
  public type via a `From` impl in the engine crate, not in the
  SPI.

## Stage 5 implementation gotchas

- `chrono` is already a transitive dep of `starter-flow-spi`
  via `starter-spi`; engine code can pull it without a new
  dependency baseline regeneration.
- The propagator's tick counter is per-run, not global. Q2
  locked that `seq` is monotonic per-run.
- `RunStore::checkpoint` takes the writes batch by slice borrow
  — the propagator must hold the batch in an owned `Vec`
  through the await point (no escape from the iterator).
- `FlowEvent::CheckpointFailed { attempt }` is emitted once per
  retry (`1..=5`); the broadcast is best-effort per D-F3.10.
- `FlowEvent::DedupShortCircuit { prior_run_id }` is emitted by
  `FlowAsService` (stage 7), not by the engine — stage 5 just
  needs to make sure the engine's `EngineError::BackendUnavailable`
  path doesn't accidentally short-circuit through the same
  variant.

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
