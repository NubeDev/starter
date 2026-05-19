# Handover — starter-flow-phase3-persistence-surfaces

## Current stage

**Stage 4 — three SQLite store impls in `crates/starter-store-sqlite/src/flow/`
behind a new default-off `flow` feature.** Stage 3 (SPI fleshout +
baseline regeneration) is complete and pushed.

## Stages complete

- **Stage 1.** D-F3.1..D-F3.12 + Q1..Q8 locked; Q4 → STANDALONE.
- **Stage 2.** REVIEW gate passed (user signed off via "keep going").
- **Stage 3.** SPI trait fleshout + baseline regeneration landed in one
  commit per D-F3.7.

## Stage 3 outcome (one-line summary)

`FlowStore` + `RunStore` + `SessionStore` trait method shapes per
D-F3.1, `RunStore::find_by_dedup_key` added per D-F3.12, additive
value types (`RunOpts`, `CheckpointRetention`, `RunState`,
`RunCheckpoint`, `RunOutcome`, `FlowRevision`, `SessionId`,
`SessionRecord`, `DedupKey`, `RunMetrics`, `EngineHealth`,
`EngineError`), additive `FlowEvent` variants (`CheckpointFailed`,
`DedupShortCircuit`), additive `FlowError::NotFound { kind, id }`,
additive `EventSink::dedup_key(&self, kind, payload) -> Option<String>`
default-`None` method. Baseline regenerated in the same commit per
D-F3.7. 11 unit tests pass; workspace builds `--all-features`;
`workspace_dep_tree_gates` test green (all 5 sub-tests including the
baseline-holds check); clippy clean across SPI-dependent crates.

## Stage 3 implementation choices made

- **`EventSink::dedup_key` signature**:
  `(&self, kind: &str, payload: &Value) -> Option<String>` (mirrors
  `emit`'s shape). Default `None` impl keeps all existing
  `impl EventSink for …` blocks source-compatible (verified via
  `cargo test -p starter-spi`).
- **`RunStore::checkpoint` gained an explicit `seq: u64` param**
  (deviation from the SCOPE prose "verbatim" block). Reason: the
  resume path uses `MAX(seq)` to find the latest checkpoint, so
  the propagator's monotonic tick counter (Q2) must drive `seq`;
  store auto-increment would lose the tick-counter ↔ seq linkage
  and break resume correctness. Documented in the trait method
  doc-comment.
- **`RunStore::start` gained an `Option<DedupKey>` param**
  (deviation from the SCOPE prose "verbatim" block). Reason:
  D-F3.12 requires dedup-key recording at run-start time inside
  the `runs` row that the `UNIQUE (service_name, dedup_key)`
  partial index protects. Non-service runs pass `None`. The
  `DedupKey` type was added to the SPI to keep the pair typed.
- **`EngineError` lives in `starter-flow-spi`** as a new
  SPI-level type, distinct from the engine crate's internal
  state-machine `EngineError` (which carries `IllegalTransition`
  for the R12 transition matrix). Variants: `BackendUnavailable`
  + `Flow(#[from] FlowError)`. Stage 6 will decide whether to
  merge or keep separate.
- **`RunState` enum** — minimal: `Running, Paused, Completed,
  Failed, Cancelled`. `Paused` included now so the checkpoint
  schema doesn't need a migration when Phase 7 lands per-flow
  pause.
- **No new `session.rs` file** — `SessionStore` co-located in
  `flow.rs` per Q1 (smaller module surface).

## Known pre-existing issues (NOT caused by stage 3)

- `cargo clippy --workspace --all-targets -- -D warnings` fails
  on master too with:
  `error[E0432]: unresolved import 'starter_grpc::testing'` in
  `crates/starter-grpc/tests/tools_service.rs` — needs
  `--features testing` which workspace clippy doesn't add.
  Worked around stage-3 verification by scoping clippy to
  SPI-touched + transitive-dependent crates (all clean).
- `cargo fmt --check` reports pre-existing drift in:
  `crates/starter-spi/src/ui/theme/mod.rs`,
  `crates/starter-ui-theme/src/{lib,routes}.rs`,
  `examples/notes/src/server.rs`. Not stage-3 files.
  Stage 10 should track these as a separate workspace-hygiene
  follow-up (out of scope for this job).

## Branch + commits

- Branch: `codeless/starter-flow-phase3-persistence-surfaces`.
- Stage 1 commit: `8407d6e` — decision lock.
- Stage 3 commit: see latest log — SPI fleshout + baseline
  regeneration.
- Pushed to origin.

## What stage 4 starts with

- New `flow` cargo feature on `starter-store-sqlite` (default-off
  per D-F3.3).
- `crates/starter-store-sqlite/src/flow/{flow_store,run_store,
  session_store}.rs` + a sibling `schema` module for JSON-envelope
  serializers.
- Migrations in `crates/starter-store-sqlite/migrations/flow/`
  following the existing `NNNN_<name>.sql` shape. First migration
  carries a header comment naming the forward-only convention
  (D-F3 durability hardening block).
- Tables: `flow_revisions`, `flow_heads`, `runs` (with
  `dedup_key TEXT NULL` + `service_name TEXT NULL` + the
  `UNIQUE (service_name, dedup_key) WHERE … IS NOT NULL` partial
  index per D-F3.12), `run_checkpoints` with `(run_id, seq)`
  primary key, `sessions`.
- WAL pragmas (`journal_mode=WAL, synchronous=NORMAL,
  busy_timeout=5000, foreign_keys=ON`) applied by **extending**
  (not replacing) the existing `crates/starter-store-sqlite/src/pool/`
  connection-init path.
- `SqliteRunStore::checkpoint` wraps the row insert + companion
  `runs.status` update + in-tx pruning in a single
  `BEGIN IMMEDIATE` transaction (D-F3.8 + D-F3.9).
- Unit tests cover the five cases named in WORKFLOW.md stage 4
  "Done when": (a) `BEGIN IMMEDIATE` atomicity via injected
  fault; (b) in-tx pruning 200→100 with `min(seq)=101`;
  (c) `finish` keep-final-row; (d) dedup-uniqueness collision;
  (e) WAL-pragma verification.

## Stage-4 implementation gotchas

- The SPI `RunStore::checkpoint` signature includes the
  `seq: u64` param; the store derives nothing from auto-increment.
- The SPI `RunStore::start` signature includes
  `dedup: Option<DedupKey>`; the store writes the pair into the
  `runs` row when `Some`, leaves both NULL when `None`.
- `chrono` is not a direct dep of `starter-flow-spi` (it's
  transitive via `starter-spi`). `created_at` / `updated_at`
  columns are populated inside the store impl (SQLite
  `CURRENT_TIMESTAMP` or a Rust-side `chrono::Utc::now()` — pick
  at impl time); they don't appear on the SPI value types.

## Phase-3 follow-up notes (not in scope for this job)

- 1M-tick long-uptime soak as a nightly CI job (Q6).
- `RunOpts.checkpoint_backoff` if a consumer surfaces a real
  need (Q8).
- `FlowEvent::HealthChanged` engine-level event once a
  Phase-7-owned engine-level event bus exists (Q7).
- `starter-store-postgres` `flow` feature mirror (D-F3.3 revisit
  trigger).
- Reconcile the two `EngineError` types (SPI vs engine-internal)
  — stage 6 decision.
- Pre-existing workspace fmt drift in theme/ui-theme/notes
  crates (out-of-scope for the Phase 3 job).
- Pre-existing `starter-grpc` clippy failure under
  `--workspace --all-targets` (needs `--features testing`).
