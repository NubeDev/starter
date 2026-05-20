# Scope — starter-flow-phase3-persistence-surfaces

> Source of truth:
> [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> §"Phase 3 — Persistence + surface wrappers" plus the R6 (sessions
> persist; runs persist; checkpoints are engine-typed), R8 (flows
> surface as Tools), and R9 (flows surface as Services) rule blocks.
> This file is the per-job brief; intentionally short. When this file
> disagrees with the source-of-truth SCOPE, that doc wins.

## Goal

Land Phase 3 of the flow SCOPE **with 24/7 supervisory-runtime
rigor** — the Niagara / long-running-station posture, not a
toy persistence layer. The engine must survive arbitrary process
kills, backend disruptions, lagging subscribers, and months of
uninterrupted ticking without unbounded resource growth or silent
data loss. The merged
[`starter-flow-engine-finish`](../starter-flow-engine-finish/) sibling
job (PR #6) closed Phase 2 honestly — `transform` + `tool_call`
bodies, the two SCOPE smoke tests, the R3 grep-contract test, and
the dep-tree gates all landed and stay green. Phase 3's deliverables
have not:

- `FlowStore` / `RunStore` / `SessionStore` trait method shapes are
  still empty seams in [`starter-flow-spi`](../../../crates/starter-flow-spi/src/flow.rs).
- No SQLite-backed impls exist for any of the three under the `flow`
  feature in
  [`starter-store-sqlite`](../../../crates/starter-store-sqlite/).
- Run checkpointing on slot writes is not wired into the engine; no
  resume-from-checkpoint path after a process restart.
- [`starter-flow-surfaces`](../../../crates/starter-flow-surfaces/src/lib.rs)
  still ships `FlowAsTool` and `FlowAsService` as empty
  public-API placeholder structs.
- The three Phase 3 SCOPE smoke tests do not exist:
  "flow invoked from MCP via `starter-mcp` unchanged",
  "flow runs as a `Service` driven by a `tokio` test channel",
  and the workspace four-transport stream smoke extended with a
  `FlowEvent` source.
- **No 24/7 durability proofs exist.** No crash-and-resume smoke;
  no checkpoint pruning to bound `run_checkpoints` table growth;
  no degraded-mode posture when the `RunStore` backend is
  unreachable; no `FlowAsService` event dedup, so a re-delivered
  event re-runs the flow; no backpressure handling for lagging
  `FlowEvent` subscribers (D1c names the per-subscriber `Lagged`
  semantics but Phase 2 had no producer fast enough to exercise
  them).

This is a **phase-completion job** scoped to exactly those five
pieces, the workspace verify that confirms the Phase 1 and
Phase 2 dep-tree gates still hold, **and the durability hardening
that makes the result fit for 24/7 supervisory deployment** (the
existing
[`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
test gains one new SPI-baseline regeneration to absorb the trait
fleshout). No engine refactors beyond the checkpoint wiring and
the durability-hardening hooks named below, no new node kinds
(Phase 5 owns the remaining built-ins), no `ai-agent` body (Phase
4 owns D1 resolution), no extension-side `contributes.nodes`
parsing (Phase 6 owns the extension adapter).

After this job lands, Phase 3 of the flow SCOPE is honestly green
**and the runtime is rated for continuous operation**: a flow
round-trips through `starter-mcp` against a real persisted store,
a flow runs end-to-end as a `Service` with at-least-once delivery
+ dedup, the workspace four-transport stream smoke covers
`FlowEvent` with backpressure semantics, a `SIGKILL`-mid-tick
process restart resumes the run from the last atomic checkpoint
with no slot-corruption, the engine continues serving in-flight
runs when the `RunStore` backend goes away and recovers cleanly
when it returns, and the `run_checkpoints` table size is bounded
by an explicit pruning policy. Phase 4 has a persistent,
operationally-rated engine to mount the `ai-agent` body on
instead of an in-memory smoke harness.

## In scope

- **SPI trait fleshout** in `crates/starter-flow-spi/src/flow.rs`
  (and a new `session.rs` if needed). `FlowStore` gains
  `load(FlowId, Option<FlowRevisionId>) -> FlowRevision`,
  `put(FlowRevision) -> FlowRevisionId`, `list() -> Vec<FlowId>`,
  `revisions(FlowId) -> Vec<FlowRevisionId>`,
  `head(FlowId) -> Option<FlowRevisionId>`. `RunStore` gains
  `start(RunId, FlowRevisionId, RunOpts, Principal) -> ()`,
  `checkpoint(RunId, RunState, &[(SlotRef, SlotValue)]) -> ()`,
  `load(RunId) -> Option<RunCheckpoint>`,
  `finish(RunId, RunOutcome) -> ()`,
  `list_open() -> Vec<RunId>`. `SessionStore` lands net-new in
  `starter-flow-spi` with `get(SessionId)`, `put(SessionId,
  SessionRecord)`, `list(Principal) -> Vec<SessionId>`. All three
  trait method bodies stay `async`; `#[non_exhaustive]` on every
  new public enum and config struct per the SCOPE's
  "What lands in `starter-flow-spi`" block. The Phase 1 baseline
  file [`starter-flow-spi-deps.baseline.txt`](../../../DOCS/flow/scope/starter-flow-spi-deps.baseline.txt)
  is regenerated in the **same commit** as the fleshout (the
  fleshout itself adds no new deps — `serde` + `async_trait` +
  `uuid` + `thiserror` are already there — but the
  [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
  test diffs against the baseline byte-for-byte, so the file is
  regenerated even if its contents are identical, to keep the
  regeneration step explicit in the audit trail).
- **SQLite store impls** in `crates/starter-store-sqlite/src/flow/`
  behind a new `flow` cargo feature on `starter-store-sqlite`,
  default-off. Three impls: `SqliteFlowStore`, `SqliteRunStore`,
  `SqliteSessionStore`. Migrations live in
  `crates/starter-store-sqlite/migrations/flow/` and follow the
  existing `NNNN_<name>.sql` shape. Schema is deliberately minimal
  per R6: `flow_revisions(flow_id, revision_id, body_json,
  created_at)`; `flow_heads(flow_id PRIMARY KEY, revision_id,
  updated_at)`; `runs(run_id, flow_revision_id, principal_json,
  run_opts_json, status, created_at, finished_at)`;
  `run_checkpoints(run_id, seq, run_state_json, slot_writes_json,
  created_at)` with `(run_id, seq)` as the primary key; `sessions
  (session_id, principal_json, body_json, created_at, updated_at)`.
  Schema details that touch wire shape (the JSON envelopes) live
  behind a single `mod schema { … }` so the migration files and the
  serializer stay in lockstep. Unit tests exercise each impl
  against the existing
  [`crates/starter-store-sqlite/src/testing/`](../../../crates/starter-store-sqlite/src/testing/)
  helpers.
- **Run checkpointing wired into the engine.** The engine's
  per-run propagator gains an `Option<Arc<dyn RunStore>>` slot
  threaded through `Engine::with_run_store(…)`. On every batch of
  `write_slot` calls inside a single propagator tick the propagator
  calls `RunStore::checkpoint(run_id, run_state, batch)` once per
  tick — not once per slot write — so the checkpoint cadence is
  bounded by tick count, not by graph fan-out. On `Engine::start`
  with a known `RunId`, the engine loads the last checkpoint via
  `RunStore::load(run_id)` and replays the slot writes through the
  same `GraphStore::write_slot` chokepoint (R2 unchanged: the
  resume path is not a second writer; the propagator's
  short-circuit on idempotent writes (D1a) absorbs the no-op writes
  that already-current slots produce). When no `RunStore` is
  attached, the engine behaves exactly as it does today (in-memory
  Phase 2 substrate).
- **`FlowAsTool` body** in
  `crates/starter-flow-surfaces/src/lib.rs`. Fields land per R8:
  `flow_id: FlowId`, `engine: Arc<Engine>`, `tool_id: KindId`,
  `name: String`, `description: String`, `input_schema:
  serde_json::Value`, `output_schema: serde_json::Value`. The
  `Tool` impl from `starter_spi::tool::Tool`
  forwards `Tool::call(args, principal, cancel, sink)` into
  `Engine::run(flow_id, args, principal, cancel)` and yields the
  flow's terminal output slot value as the tool's return. Span on
  `flow_as_tool.call` records (flow_id, tool_id, principal_id_hash,
  run_id). Errors from the engine map to a typed `ToolError` per
  the `starter-spi` contract.
- **`FlowAsService` body** in
  `crates/starter-flow-surfaces/src/lib.rs`. Fields land per R9:
  `flow_id: FlowId`, `engine: Arc<Engine>`, `service_name: String`,
  `event_sink: Arc<dyn EventSink>`. The `Service` impl from
  `starter_spi::service::Service` subscribes to the `EventSink` on
  `Service::start` and invokes the flow once per received event;
  the per-event run uses the event's `Principal` (or the service's
  configured default principal if the event carries none).
  `Service::stop` cancels in-flight runs via the engine's `Cancel`
  seam and drains. Span on `flow_as_service.invoke` records
  (flow_id, service_name, event_id, run_id).
- **Four Phase 3 SCOPE smoke tests** as integration tests:
  - `crates/smoke-tests/tests/flow_via_mcp.rs` —
    `FlowAsTool`-wrapped flow registered with `starter-mcp`'s
    existing `ToolRegistry`; an MCP client (the same in-process
    test harness `starter-mcp`'s own tests use) calls the flow by
    its `tool_id`; assertion: the MCP client receives the flow's
    terminal output and the run is recorded in a `SqliteRunStore`
    with a non-empty checkpoint history. This smoke lives in
    `crates/smoke-tests/` rather than `crates/starter-flow/tests/`
    because it crosses crate boundaries (engine + surfaces + mcp +
    sqlite store) — the D1d ownership rule applies in the opposite
    direction here, which is exactly the revisit-trigger D1d names.
  - `crates/smoke-tests/tests/flow_as_service.rs` —
    `FlowAsService`-wrapped flow whose event source is a
    `tokio::sync::mpsc` channel the test owns; the test pushes
    three events, asserts three runs land in `SqliteRunStore`, all
    three reach the `Finished` state, and `Service::stop` drains
    cleanly. Includes a re-delivery sub-case per D-F3.12:
    pushes the same event twice with the same dedup key,
    asserts only one `runs` row, asserts the second
    `EventSink::recv` returns the first run's outcome, and
    asserts a `FlowEvent::DedupShortCircuit` was emitted on
    the run's broadcast. Same cross-crate justification.
  - The existing workspace four-transport stream smoke (REST SSE,
    MCP, gRPC streaming, JSON-RPC stdio — wherever it currently
    lives under `crates/smoke-tests/`) is **extended** with a
    `FlowEvent` source: one new source row in the smoke's
    parameterised matrix, plus the small adapter wiring needed to
    drive `FlowRun::subscribe()` into the existing four-transport
    harness. Adds one new sub-case per transport: a deliberately-
    lagging consumer (`sleep(50ms)` per event under a 1ms
    producer cadence) asserts non-zero `subscriber_lagged_count`
    on the run's `RunMetrics` while the run itself still
    finishes successfully (D-F3.10 backpressure invariant).
    If the four-transport smoke doesn't yet exist under
    `crates/smoke-tests/` (stage 1 verifies), stage 9 instead
    lands a standalone `crates/smoke-tests/tests/
    flow_event_stream_over_four_transports.rs` covering the same
    matrix with only `FlowEvent` as the source — bias toward
    extending the existing file when possible.
  - `crates/smoke-tests/tests/flow_crash_and_resume.rs` — the
    24/7 durability smoke. Three sub-cases. **(a)** Build a
    flow that ticks 50 times and checkpoints per-tick via
    `SqliteRunStore` on a file-backed database (not
    `:memory:`); at tick 25 the test process spawns a child
    that runs the flow, sends `SIGKILL` to the child after
    the 25th checkpoint commits (verified by polling the
    `run_checkpoints` table for `seq=25`), then on the parent
    process opens the same database and resumes the run via
    `Engine::start(known_run_id)`; assert the run finishes
    at tick 50 with no slot-corruption and exactly
    `50 - 25 = 25` additional checkpoints written.
    **(b)** Inject a 10-second `RunStore` backend outage
    mid-tick (via a wrapped `RunStore` that returns errors
    during the window); assert the engine emits
    `FlowEvent::CheckpointFailed` per retry attempt, transitions
    to `EngineHealth::Degraded` after 5 consecutive failures,
    keeps producing `SlotChanged` events for in-flight runs,
    rejects new `Engine::start` calls with
    `EngineError::BackendUnavailable`, and on outage clear
    drains the queued checkpoint batches in `(run_id, seq)`
    order and transitions back to `EngineHealth::Healthy`.
    **(c)** A short soak: tick a small flow 10000 times in-
    memory and assert tick-counter monotonicity, no tokio task
    leaks, and bounded resident-set-size growth (within 2× the
    per-tick allocation count measured at tick 100). Same
    cross-crate justification — needs the SQLite store plus
    the engine plus a process-spawn harness.
- **Workspace verify + dep-tree gates re-confirmed.** `cargo build
  --workspace --all-features`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`. The existing
  [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
  test still passes after the SPI-baseline regeneration. New
  invariant checked in the same test (one new assertion): the four
  flow crates (`starter-flow-spi`, `starter-flow`,
  `starter-flow-nodes`, `starter-flow-surfaces`) still do not
  path-dep onto `starter-mcp` / `starter-server` / `starter-cli`
  — Phase 3 wires the other way: the surfaces crate exposes traits
  the host (`starter-mcp` etc.) consumes, not the reverse.
  `starter-store-sqlite`'s new `flow` feature must be a strict
  superset (no default-feature spillover) and must not pull
  `starter-mcp` / `starter-server` / `starter-cli` into the dep
  tree of any flow crate.
- **Durability hardening for 24/7 operation** (the load-bearing
  difference between "Phase 3 compiles" and "Phase 3 deploys"):
  - **Atomic checkpoint writes.** `SqliteRunStore::checkpoint`
    wraps the `run_checkpoints` row insert and any companion
    `runs.status` update in a single SQL transaction
    (`BEGIN IMMEDIATE … COMMIT`). A crash mid-write either
    leaves the prior checkpoint as the latest, or commits the
    new one atomically — never a partial row.
    `journal_mode=WAL`, `synchronous=NORMAL` (the SQLite
    durability sweet-spot for high-write workloads; locked in
    D-F3.8).
  - **Bounded checkpoint history.** A pruning policy keeps the
    `run_checkpoints` table from growing unbounded under
    long-running flows. Default: keep the last 100 checkpoints
    per open run plus the final checkpoint per finished run;
    configurable via `RunOpts.checkpoint_retention`. Pruning
    runs lazily after each successful checkpoint write inside
    the same transaction (D-F3.9).
  - **Backend-failure posture.** When `RunStore::checkpoint` or
    `RunStore::finish` returns an error, the engine emits
    `FlowEvent::CheckpointFailed { run_id, error, attempt }`
    on the run's broadcast and retries with exponential backoff
    (50ms → 100ms → 200ms → 400ms → 800ms; capped at 5
    attempts). After 5 consecutive failures the run is marked
    `Degraded`; the engine continues serving the run from
    in-memory state, refuses to start new runs (returns a typed
    `EngineError::BackendUnavailable`), and emits a single
    health-state event. When a subsequent checkpoint succeeds
    the engine clears `Degraded` and resumes accepting new
    runs. Process exit during `Degraded` is non-destructive:
    the in-memory in-flight runs are lost, but the last
    durable checkpoint is intact and resume-from-checkpoint
    picks up there (D-F3.11).
  - **At-least-once event delivery with dedup.**
    `FlowAsService` writes a dedup key per event into
    `runs.dedup_key` under a `UNIQUE (service_name, dedup_key)`
    index. A re-delivered event whose key already exists
    short-circuits to the prior run's outcome rather than
    starting a new run. Events with no carried key fall back
    to a hash of `(service_name, event_id, event_payload_hash)`
    computed by `FlowAsService` itself (D-F3.12).
  - **Backpressure on `FlowEvent` broadcast.** The per-run
    `tokio::sync::broadcast::Sender<FlowEvent>` capacity is set
    via `RunOpts.event_broadcast_capacity` (default 1024). A
    subscriber that lags past the capacity sees `Lagged(n)` on
    its next `recv` (standard tokio semantics, named in D1c);
    the run records a counter `subscriber_lagged_count` on
    the run-level metrics surface. Producers (the propagator,
    the engine state-machine, the surface wrappers) never block
    on broadcast send — a full channel evicts the oldest event
    for the slowest subscriber, never the producer (D-F3.10).
  - **Long-uptime invariants checked by tests.** A focused
    soak-style test ticks a small flow 10000 times under the
    in-memory store and asserts: resident set size growth is
    bounded (within 2× the per-tick allocation count measured
    at tick 100); the propagator's tick counter stays monotonic
    and uses `u64` (no `u32` wraparound risk over years of
    uptime — `u64` at 1kHz wraps in ~584 million years); no
    tokio task leaks (measured via the existing workspace
    helper if present, otherwise via a tracing-span open/close
    count).
  - **Schema migration safety.** The Phase 3 migrations under
    `crates/starter-store-sqlite/migrations/flow/` are
    forward-only and never rewrite existing rows. Future Phase
    3+ migrations that need to backfill a column do so in a
    second migration that reads existing rows and writes a new
    column — never destructively rewriting. The stage-4 commit
    documents this convention in a header comment on the first
    migration file.

## Out of scope

- **Phase 4 work** — `ai-agent` body (D1 resolution). The kind
  module in
  [`crates/starter-flow-nodes/src/ai_agent.rs`](../../../crates/starter-flow-nodes/src/ai_agent.rs)
  stays a `KIND_ID` constant; no `AiRunner` wiring lands here. The
  next job after this one drafts it.
- **Phase 5 work** — remaining built-in node kinds (`branch`,
  `merge`, `gate`, `subflow`, `trigger.{explicit, event, schedule,
  webhook}`, `http-out`, `log`, `sleep`).
- **Phase 6 work** — `starter-ext-flow` adapter for extension-
  contributed node kinds and flows. The surfaces this job ships are
  consumed by `starter-mcp` directly; the extension-adapter wiring
  is its own job.
- **Phase 7 work** — three-level stop + safe-state engine APIs
  beyond what Phase 2 already shipped (`Engine::stop` calling the
  safe-state walk is already proven by
  [`smoke_engine_is_reader_of_policies`](../../../crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs)).
  Phase 7's per-flow `pause` / `resume` APIs and per-flow safe-state
  walk do **not** land here.
- **Engine refactors.** The merged Phase 2 engine substrate is what
  this job sits on. The checkpoint wiring is a **single additional
  method on the propagator** (`maybe_checkpoint(tick_writes)`)
  plus an `Engine::with_run_store(…)` builder hook plus the
  resume-on-start path; touching `propagator.rs` / `engine.rs` for
  anything beyond those three surfaces is out. If a real defect
  surfaces, file an issue + a separate PR.
- **A visual canvas (`starter-ui-flow`).** Phase 8.
- **Hot-reload of flow definitions** (open question D3 in the flow
  SCOPE). Defer.
- **Per-flow rate limit / cost cap** (open question D4 in the flow
  SCOPE). Defer.
- **Adding the `starter-smoke-tests` crate to flow-internal smokes.**
  D1d still holds: engine-internal behaviour proofs stay in
  `crates/starter-flow/tests/`. The three Phase 3 smokes are
  cross-crate and therefore correctly live in `crates/smoke-tests/`
  — exactly the revisit-trigger D1d names.
- **Modifications to the `starter-flow-spi` baseline beyond the
  one-shot regeneration in stage 3.** Stages 4–9 must not produce a
  baseline diff; stage 9 verifies.

## Hard rules (load-bearing — inherited from flow SCOPE)

Restated so the runner re-reads them every stage:

- **R2 — One write chokepoint.** The checkpoint-resume path
  replays through `GraphStore::write_slot`, not around it. The
  existing
  [`smoke_one_write_chokepoint`](../../../crates/starter-flow/tests/smoke_one_write_chokepoint.rs)
  must still pass; stage 5 adds a focused unit test that the resume
  path's writes also enter the chokepoint.
- **R3 — Engine reads policies, never owns them.** The existing
  [`r3_no_policy_match_arms`](../../../crates/starter-flow/tests/r3_no_policy_match_arms.rs)
  must stay green through the checkpoint wiring; the propagator's
  new `maybe_checkpoint` method cannot match on policy slot names.
- **R5 — Node behaviours are stateless.** The checkpoint envelope
  records `RunState` + per-batch slot writes; it does **not**
  record node-local state. A node that needs state across ticks
  reads it from a slot — same as today.
- **R6 — Sessions persist; runs persist; checkpoints are engine-
  typed.** The three SQLite store impls are the executable form of
  R6. The checkpoint schema is the engine's `RunState` JSON plus
  the batch of slot writes that produced this revision of the run
  state — no external dep gets to define the checkpoint shape.
- **R8 — Nodes are not Tools; Tools are one node kind.**
  `FlowAsTool` exposes a flow as a `Tool` to the outside; the
  flow's internal `tool-call` nodes remain the only callers of
  `Tool::call`. `FlowAsTool` does not bypass the engine to call
  internal tools directly.
- **R9 — Flows are first-class Tools and first-class Services.**
  `FlowAsService` is the executable form of R9; the smoke at
  stage 8 is the contract.
- **R10 — Reverse-DNS ids enforced.** `FlowAsTool.tool_id`
  validates as a `KindId`. `FlowAsService.service_name` follows
  the same reverse-DNS namespace ownership convention the
  workspace SCOPE applies to service names.
- **R13 — Streaming + cancellation + observability.** The four-
  transport extension at stage 8 is the load-bearing case: a
  `FlowEvent` stream survives the same four transports the
  existing smoke covers, with the same cancellation semantics.

## Constraints

- **No new top-level deps on `starter-flow-spi`.** The trait
  fleshout uses `async_trait`, `serde`, `uuid`, `thiserror` — all
  already present. Stage 3 verifies via the regenerated baseline.
- **No `adk-rust` anywhere.** Continues to hold under the existing
  [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
  test.
- **`default-features = []` posture stays.** The new `flow`
  feature on `starter-store-sqlite` is default-off. `starter-flow`
  itself gains no new feature flags.
- **`starter-flow-surfaces` depends only on `starter-flow-spi` +
  `starter-spi` + `starter-flow`.** It does not depend on
  `starter-store-sqlite` (the store impls are wired by the host,
  not by the surfaces crate).
- **MSRV 1.78** (workspace). `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo fmt --check`
  non-negotiable.
- **Tests live with the code.** Stage 3 commits SPI trait fleshout
  + the baseline regeneration in one commit. Stage 4 commits each
  store impl + its unit tests in the same commit. Stage 5 commits
  the engine checkpoint wiring + its unit test. Stages 6 and 7
  commit the surface body + its unit tests. Stage 8 commits the
  three smokes (one commit per smoke file is acceptable; bundling
  is not).
- **One logical batch per stage.** Stage 3 = SPI fleshout. Stage 4
  = three SQLite store impls. Stage 5 = checkpoint wiring +
  resume. Stage 6 = durability hardening (atomic-tx assertions,
  retry-with-backoff, `Degraded` mode, backpressure semantics,
  monotonic tick counter). Stage 7 = `FlowAsTool`. Stage 8 =
  `FlowAsService` with dedup. Stage 9 = four Phase 3 smokes.
  Stage 10 = workspace verify. No bundling.
- **Phase 3 smokes live in `crates/smoke-tests/`,** not in
  `crates/starter-flow/tests/`. D1d revisit-trigger applies
  precisely here: these smokes need `starter-mcp`,
  `starter-store-sqlite`, the four-transport harness, and a
  process-spawn harness for the crash-and-resume smoke — all
  consumer crates `starter-flow` cannot depend on without a
  cycle.
- **Soak budget on PR CI.** The 10000-tick soak case in the
  crash-and-resume smoke runs under 10 seconds on a fresh
  laptop. A 1M-tick true-long-uptime soak belongs in a nightly
  CI job (out of scope for this job; a follow-up wires that
  job).

## Decisions

Locked in stage 1. Each decision lists the rule it derives from
and the **revisit trigger** — the event that should reopen the
question. Anything else is noise.

### D-F3.1 — `FlowStore` / `RunStore` / `SessionStore` method shapes

The Phase 3 trait fleshout takes the methods listed in the In-scope
"SPI trait fleshout" block verbatim. All methods are `async`,
take `&self`, and return concrete `Result<T, FlowError>` where
the error type is the existing
[`FlowError`](../../../crates/starter-flow-spi/src/flow.rs) enum
(extended with a `NotFound { kind, id }` variant — additive,
`#[non_exhaustive]` already present). `RunStore::checkpoint` takes
its slot-writes batch by `&[(SlotRef, SlotValue)]` rather than an
owned `Vec` so the propagator can checkpoint without allocating.

- **Why.** Matches the CRUD shape the SCOPE "Decisions made" block
  named (`load`, `put`, `list`, `revisions`, `head` for flows;
  `start`, `checkpoint`, `load`, `finish`, `list_open` for runs).
  `SessionStore` follows the same minimal CRUD posture. The
  fleshout is *additive* — every existing Phase 1 consumer (the
  `FlowStore: Send + Sync + 'static {}` empty trait) stays
  compatible because the empty trait had no methods to break.
- **Implication for stage 3.** One commit: trait fleshout +
  baseline regeneration. Stages 4–9 must not produce baseline
  diffs.
- **Revisit when.** A consumer (Codeless, Rubix) surfaces a need
  for a method the minimal shape doesn't cover (likely candidates:
  `prune_old_checkpoints`, `list_by_principal`). Those land as
  additive methods in a follow-up SPI bump with their own
  baseline regeneration.

### D-F3.2 — Checkpoint cadence: per tick, not per slot write

`RunStore::checkpoint` is invoked **once per propagator tick**
with the batch of slot writes that occurred during the tick — not
once per `GraphStore::write_slot` call. A tick that writes one
slot checkpoints one slot; a tick that fans out to ten downstream
nodes checkpoints all ten in one row.

- **Why.** Per-write checkpointing fights R2's idempotent-write
  short-circuit (D1a) and produces N rows for a single logical
  unit of work. Per-tick checkpointing matches the propagator's
  natural batch boundary, keeps the `run_checkpoints` table small
  for high-fan-out graphs, and gives the resume path a clean
  per-tick replay shape. The `(run_id, seq)` primary key on
  `run_checkpoints` uses the propagator's tick counter as `seq`.
- **Implication for stage 5.** The propagator gets a
  `maybe_checkpoint(&self, tick_writes: &[(SlotRef, SlotValue)])`
  method called at the end of each tick. If no `RunStore` is
  attached the method is a no-op.
- **Revisit when.** A consumer needs sub-tick durability
  guarantees (e.g. an `ai-agent` node mid-LLM-call that must
  survive a process crash). At that point the per-node body opts
  into a sub-tick checkpoint via a separate `RunStore` extension
  method; the default cadence stays per-tick.

### D-F3.3 — SQLite store: feature-gated crate-internal module

The three SQLite impls live in
`crates/starter-store-sqlite/src/flow/{flow_store.rs,
run_store.rs, session_store.rs}` behind a new `flow` cargo
feature on `starter-store-sqlite` (default-off). Migrations live
in `crates/starter-store-sqlite/migrations/flow/` and are gated
on the same feature so a store consumer that does not enable
`flow` does not pay the migration cost.

- **Why.** Matches the existing crate's posture (compare
  [`crates/starter-store-sqlite/src/`](../../../crates/starter-store-sqlite/src/)
  module layout). Default-off keeps the workspace baseline build
  (`cargo build --workspace`) from pulling the flow schema into
  every binary; `--all-features` exercises it in CI. The host
  (a binary built on starter that wants persisted flows) opts in
  with `starter-store-sqlite = { features = ["flow"] }`.
- **Implication for stage 4.** Each impl ships with its own
  unit-test module using the existing
  [`crates/starter-store-sqlite/src/testing/`](../../../crates/starter-store-sqlite/src/testing/)
  helpers (in-memory `:memory:` connections, migration application
  in test setup). The three impls share a `schema` module that
  serializes the JSON envelopes — keep the migration files and
  the serializer in lockstep by source-of-truth-ing the JSON
  shape from `starter-flow-spi` types.
- **Revisit when.** A consumer needs a non-SQLite backend
  (Postgres for multi-tenant deployments — the
  [`starter-store-postgres`](../../../crates/starter-store-postgres/)
  crate would mirror this layout under its own `flow` feature in
  a future job). The trait shape from D-F3.1 already accommodates
  this; no SPI change required.

### D-F3.4 — `FlowAsTool` schema source: explicit, not derived

`FlowAsTool.input_schema` and `FlowAsTool.output_schema` are
provided **explicitly** at construction time by the host, not
derived from the flow's revision body. The constructor signature
is `FlowAsTool::new(flow_id, engine, tool_id, name, description,
input_schema, output_schema)`.

- **Why.** Flow revisions may not carry a JSON schema yet
  (Phase 5's `transform` body decision deferred richer typing);
  forcing schema derivation at this stage either ships a synthetic
  empty schema (useless for MCP discovery) or blocks Phase 3 on
  Phase 5. Explicit schemas at the wrapper boundary let the host
  document the flow's input/output contract for MCP / REST / CLI
  discovery without coupling to flow-internal typing.
- **Implication for stage 6.** The smoke at stage 8 constructs a
  `FlowAsTool` with hand-written `input_schema` /
  `output_schema` JSON literals; the test asserts MCP discovery
  surfaces them unmodified.
- **Revisit when.** Phase 5 lands a richer typed `transform` body
  with schema introspection; at that point `FlowAsTool::new` gains
  a sibling `FlowAsTool::from_revision(flow_id, engine, …)` that
  derives schemas from the revision. The explicit constructor
  stays as the lower-level seam.

### D-F3.5 — `FlowAsService` event-sink coupling: subscribe-on-start

`FlowAsService` subscribes to its `EventSink` on `Service::start`
(not at construction time) and unsubscribes on `Service::stop`.
The subscription is a `tokio::spawn`-ed task owned by the service
that loops on event receipt and invokes the engine per event.

- **Why.** Matches the workspace's existing `Service` lifecycle
  convention — services that subscribe at construction time leak
  task handles when the host builds N services before starting any
  of them. `Service::stop` cancels the task via the engine's
  `Cancel` seam and joins it, so a stopped service holds no
  background work.
- **Implication for stage 7.** The smoke at stage 8 drives the
  full `start → push 3 events → stop` cycle and asserts the
  spawned task is joined cleanly (no leaked tokio task — verify
  via the existing workspace test helper if one exists, otherwise
  via a tracing assertion on the `flow_as_service.invoke` span
  closure count).
- **Revisit when.** A consumer needs the subscription to survive
  `Service::stop / Service::start` cycles (e.g. for buffered
  event replay). At that point the `EventSink` contract grows a
  resume cursor; the service-side change is small.

### D-F3.6 — Phase 3 smoke location: `crates/smoke-tests/`

The three Phase 3 SCOPE smokes live under
`crates/smoke-tests/tests/`, not under
`crates/starter-flow/tests/`. The D1d revisit-trigger names this
case precisely: each Phase 3 smoke needs node kinds or surfaces
from a crate `starter-flow` cannot depend on without a cycle
(`starter-mcp`, `starter-store-sqlite`, the four-transport
harness). The `starter-smoke-tests` crate is the only workspace
member with the dep-graph reach to host them.

- **Why.** D1d named this exact case as its revisit trigger;
  Phase 3 is the moment that trigger fires. Engine-internal
  smokes from Phase 2 stay where they are (the
  [`r3_no_policy_match_arms`](../../../crates/starter-flow/tests/r3_no_policy_match_arms.rs)
  test, the
  [`smoke_one_write_chokepoint`](../../../crates/starter-flow/tests/smoke_one_write_chokepoint.rs)
  test, the
  [`smoke_engine_is_reader_of_policies`](../../../crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs)
  test) — they prove engine-internal contracts on the engine
  crate's own tree. Cross-crate smokes go where the dep graph
  allows them to compile.
- **Implication for stage 9.** The four smoke files land as
  siblings under `crates/smoke-tests/tests/`; their names mirror
  the SCOPE Phase 3 Smoke block plus the durability addendum:
  `flow_via_mcp.rs`, `flow_as_service.rs`, (extension or
  standalone) `flow_event_stream_over_four_transports.rs`,
  and `flow_crash_and_resume.rs` (the 24/7 durability proof).
- **Revisit when.** A future cross-crate smoke needs a crate the
  `starter-smoke-tests` Cargo.toml does not yet pull in. That's
  a one-line dep addition, not a SCOPE change.

### D-F3.7 — SPI-baseline regeneration discipline

The Phase 1 baseline file at
[`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`](../../../DOCS/flow/scope/starter-flow-spi-deps.baseline.txt)
is regenerated **once, in stage 3's single commit**, even if the
fleshout adds no new deps. The regeneration command is documented
inline in the file (matching the existing comment header
convention; if no comment exists, stage 3 adds one). Stages 4–9
that produce a baseline diff are a stage-fail; revert.

- **Why.** Keeps the regeneration step explicit in the audit
  trail. A future SPI bump should not have to guess whether the
  baseline was last regenerated. The Phase 1 baseline file
  pre-dates this job; treating it as a snapshot maintained
  alongside intentional SPI changes (not a drift detector) is the
  posture the
  [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
  test enforces.
- **Implication for stage 3.** The commit message names the
  regeneration explicitly. The commit's diff includes both the
  trait fleshout AND the baseline file (even if the baseline
  bytes happen to be identical).
- **Revisit when.** The baseline file grows a header convention
  that names a regeneration command; at that point the convention
  is documented in the SCOPE rather than restated per-job.

### D-F3.8 — Atomic checkpoint writes (WAL + IMMEDIATE tx)

`SqliteRunStore::checkpoint` opens a `BEGIN IMMEDIATE` transaction,
inserts the `run_checkpoints` row, updates the `runs.status` row,
runs the pruning step (D-F3.9), and commits. On any error inside
the transaction the rollback leaves the prior checkpoint as the
latest visible state. The SQLite pragmas applied at connection
init are `journal_mode=WAL`, `synchronous=NORMAL`,
`busy_timeout=5000`, `foreign_keys=ON`. These are set in the
existing
[`crates/starter-store-sqlite/src/pool/`](../../../crates/starter-store-sqlite/src/pool/)
connection-init path (extend, not replace).

- **Why.** WAL + `synchronous=NORMAL` is the documented SQLite
  posture for write-heavy long-running workloads — survives
  process crashes (the WAL is fsynced at commit boundaries) at
  the cost of a possible loss of the most recent transaction on
  power-loss. For starter-flow's per-tick checkpoint cadence
  (D-F3.2) this is the right trade: a power-loss may lose the
  last tick, but never corrupt the database. `synchronous=FULL`
  would force an fsync per commit and halve throughput; the
  per-tick batch already amortises commit cost.
  `BEGIN IMMEDIATE` (rather than `BEGIN DEFERRED`) takes the
  write lock at transaction start, eliminating the
  `SQLITE_BUSY` retry storm that a deferred transaction races
  with concurrent readers (the `RunStore::load` resume path).
- **Implication for stage 4.** Each `SqliteRunStore::checkpoint`
  call is one transaction. The stage-4 unit tests assert
  atomicity by injecting a fault between the row insert and
  the commit (via a wrapped connection that panics on the
  second statement) and verifying the prior checkpoint is still
  the latest visible state. The stage-6 durability stage adds
  the crash-and-resume integration test that exercises the
  same invariant end-to-end.
- **Revisit when.** A consumer needs the strict-fsync posture
  (e.g. a regulatory environment that forbids losing any
  acknowledged write). At that point `synchronous=FULL`
  becomes a `RunOpts.checkpoint_durability` opt-in; the
  default stays `NORMAL`.

### D-F3.9 — Bounded checkpoint history (lazy pruning)

The `run_checkpoints` table is kept bounded by an in-transaction
pruning step at the end of `SqliteRunStore::checkpoint`. The
default policy keeps the **last 100 checkpoints per open run**
plus the **final checkpoint per finished run** (so resume from
the most recent state is always possible; historical replay
beyond 100 ticks for an open run is not). The policy is
configurable via `RunOpts.checkpoint_retention` on a per-run
basis. Pruning runs inside the same transaction as the insert
so the table never momentarily exceeds the bound.

- **Why.** A flow that ticks for a year at 1Hz produces ~31
  million `run_checkpoints` rows without a cap. Resume only
  needs the latest row (`MAX(seq)` per `run_id`); historical
  intermediate states are not load-bearing for correctness.
  100 is a soak-buffer for replay-debugging without unbounded
  growth; consumers that need more set it on `RunOpts`. Lazy
  pruning (inside the write transaction) is simpler than a
  background sweep and avoids a second `RunStore` method or a
  separate ticker.
- **Implication for stage 4.** A `DELETE FROM run_checkpoints
  WHERE run_id = ? AND seq <= ?` runs after the insert with
  `? = (MAX(seq) - retention)`. The stage-4 unit tests assert
  that after 200 checkpoints on a run with `retention=100` the
  table has exactly 100 rows for that run and the lowest `seq`
  is `101`. The finished-run final-row preservation lives in
  `SqliteRunStore::finish` as a one-shot delete-all-but-max.
- **Revisit when.** A consumer needs full historical replay
  for audit (e.g. a regulatory environment that requires the
  full slot-write history). At that point `RunOpts.
  checkpoint_retention = Unbounded` is the opt-in;
  `Bounded(100)` stays the default. Or a separate
  `RunStore::archive(run_id, sink)` method ships in a follow-up
  to ship pruned rows to a cold-storage backend before
  deletion.

### D-F3.10 — `FlowEvent` broadcast backpressure (evict-oldest)

The per-run `tokio::sync::broadcast::Sender<FlowEvent>` is
constructed with capacity from `RunOpts.event_broadcast_capacity`
(default **1024**). Producers (the propagator, the engine
state-machine, the surface wrappers) call `Sender::send` which
under tokio broadcast semantics never blocks — a full channel
silently overwrites the slot for the slowest consumer, and that
consumer sees `RecvError::Lagged(n)` on its next `recv`. The
engine attaches one per-run subscriber of its own that monitors
for `Lagged` and increments a `subscriber_lagged_count` counter
on the run's metrics surface; the count is exposed via a new
`Engine::run_metrics(run_id) -> RunMetrics` accessor.

- **Why.** D1c locked `tokio::sync::broadcast` as the cardinality
  shape; this decision locks how the engine handles the
  inevitable lag a slow REST-SSE subscriber will produce against
  a fast propagator. Blocking the producer would back-pressure
  the entire engine off a single slow consumer (catastrophic for
  24/7); silently dropping events without surfacing the loss
  would violate observability (R13). Evict-oldest plus a
  counter that surfaces in metrics is the standard tokio
  posture and matches how Niagara-style supervisory runtimes
  treat slow-consumer subscriptions (the alarm console missing
  a frame is preferable to the whole station hanging).
- **Implication for stages 7+8.** The four-transport smoke
  at stage 9 includes one parameterised row where the consumer
  deliberately lags (`sleep(50ms)` per event under a 1ms
  producer cadence) and asserts the run's
  `subscriber_lagged_count` ends non-zero while the run itself
  finishes successfully. The other transport rows assert the
  counter ends zero (no spurious lag under normal load).
- **Revisit when.** A consumer needs lossless event streaming
  (e.g. an audit log feeding a compliance store). At that point
  the audit consumer subscribes via a separate persistent
  `FlowEvent` sink fed from the `RunStore::checkpoint` write
  path (every persisted event is recoverable), not from the
  broadcast — broadcast is for live observers, not the
  durable record.

### D-F3.11 — `RunStore` backend-failure posture (Degraded mode)

When `RunStore::checkpoint` or `RunStore::finish` returns an
error, the engine emits `FlowEvent::CheckpointFailed { run_id,
error: String, attempt: u32 }` on the run's broadcast and
retries with exponential backoff capped at 5 attempts:
`50ms, 100ms, 200ms, 400ms, 800ms`. After 5 consecutive
checkpoint failures across **any** open run, the engine
transitions to `EngineHealth::Degraded`: in-flight runs continue
serving from in-memory state (their checkpoint queue accumulates
in memory bounded by `RunOpts.degraded_queue_capacity`, default
1024 batches), but `Engine::start(new_run)` returns
`EngineError::BackendUnavailable`. A successful checkpoint on
any run drains the queued batches in `(run_id, seq)` order and
transitions the engine back to `EngineHealth::Healthy`. The
queue capacity is per-run, evict-oldest on overflow (same
posture as D-F3.10's broadcast) with a `degraded_dropped_count`
counter that surfaces on `RunMetrics`. Process exit during
`Degraded` is non-destructive: the in-memory queue is lost, but
the last durable checkpoint is intact; resume-from-checkpoint
on the next process start picks up there.

- **Why.** A 24/7 runtime cannot be the kind of system where a
  10-second SQLite-backend hiccup terminates every in-flight
  flow. Niagara-style supervisory platforms degrade gracefully:
  control loops keep running off cached state when the
  historian's down; the historian fills the gap on
  reconnection. Same posture here: the in-memory queue absorbs
  the gap, the engine surfaces health prominently, and consumers
  with their own policies decide whether to keep dispatching
  new work to a degraded engine. The evict-oldest queue
  posture matches D-F3.10 — bounded memory under degraded
  operation is non-negotiable.
- **Implication for stage 6 (durability).** The crash-and-
  resume smoke at stage 6 includes a sub-case that injects a
  10-second backend outage mid-tick, asserts the engine enters
  `Degraded`, asserts the in-flight run keeps producing
  `FlowEvent::SlotChanged` events, asserts the queued batches
  drain on backend return, and asserts the engine transitions
  back to `Healthy`. A second sub-case kills the process during
  `Degraded` and asserts the resume path picks up the last
  durable (pre-outage) checkpoint without seeing the queued-
  in-memory writes.
- **Revisit when.** A consumer needs strict-stop on backend
  failure (e.g. a system where running off stale state is
  worse than stopping). At that point a `RunOpts.
  on_backend_failure: AbortRun | Degrade` opt-in lands;
  `Degrade` stays the default.

### D-F3.12 — `FlowAsService` at-least-once delivery + dedup

`FlowAsService` writes a dedup key per event into a new
`runs.dedup_key TEXT` column under a `UNIQUE (service_name,
dedup_key)` index. Per-event dedup-key resolution order:
(1) the event's own `dedup_key()` accessor if the `EventSink`
contract carries one; (2) a fallback hash of `(service_name,
event_id, blake3(event_payload_bytes))` computed by
`FlowAsService`. On `EventSink::recv`, `FlowAsService` first
checks `RunStore` for a prior run with the same
`(service_name, dedup_key)`; if found, emits
`FlowEvent::DedupShortCircuit { prior_run_id }` on the
service's observer and returns the prior run's outcome
without starting a new run. Otherwise the new run starts
normally with the dedup key recorded at `Engine::start` time.

- **Why.** At-least-once event delivery is the only realistic
  contract over any of the four transports (REST SSE
  reconnects, MCP retries, gRPC streaming reconnects,
  JSON-RPC stdio reconnects can all redeliver). Without
  dedup, a re-delivered "send email" event sends two emails;
  with dedup, the second delivery returns the first run's
  outcome. The `UNIQUE` index makes the dedup race-free at
  the database level (a concurrent second invocation collides
  on insert and reads the first's row). `blake3` is fast and
  already vetted by the workspace; an `EventSink`-provided
  key is preferred because the producer knows the semantic
  unit of work better than the wrapper does.
- **Implication for stage 8.** `FlowAsService` constructor
  gains no new field — the dedup-key resolution is per-event
  internal logic. The `EventSink` SPI doesn't grow a new
  method in this job; the per-event `dedup_key()` accessor is
  an optional method with a default `None` implementation on
  the existing trait (additive). The stage-9 `flow_as_service`
  smoke includes a sub-case that re-delivers an event with
  the same dedup key and asserts: only one run lands in
  `RunStore`, the second `EventSink::recv` returns the first
  run's outcome, and a `FlowEvent::DedupShortCircuit` is
  emitted.
- **Revisit when.** A consumer needs at-most-once semantics
  (e.g. a financial transaction that must error rather than
  short-circuit on re-delivery). At that point a `FlowAsService
  ::with_delivery(DeliveryMode::AtMostOnce)` opt-in lands;
  `AtLeastOnce` stays the default.

## Cross-cutting checks the runner must keep honest

- **R2 chokepoint integrity through resume.** Stage 5's unit test
  attaches a tracing subscriber that counts `write_slot` span
  entries during a resume-from-checkpoint cycle; the count equals
  the checkpoint's slot-writes batch size. If the resume path
  bypasses `write_slot`, the test fails.
- **R3 policy-discipline through the new propagator method.** The
  existing
  [`r3_no_policy_match_arms`](../../../crates/starter-flow/tests/r3_no_policy_match_arms.rs)
  test must stay green after stage 5. Run it locally before commit.
- **R5 `&self` discipline preserved.** The engine's
  `with_run_store` builder hook and the propagator's
  `maybe_checkpoint` method both take `&self` (or, where the
  builder needs mutation, returns `Self` per the existing builder
  convention). No `&mut self` slips into the propagator's
  per-invoke API.
- **No `adk-rust`.** Continues to hold via the existing dep-tree
  gates test.
- **`-spi` baseline matches after stage 3.** Stages 4–9 must not
  drift the baseline.
- **`FlowEvent` cardinality preserved (D1c).** The four-transport
  smoke at stage 8 must not collapse the per-run broadcast into a
  single-consumer mpsc — multi-consumer is the contract the four
  transports presuppose.
- **No `starter-flow-surfaces` → `starter-store-sqlite` dep.**
  The host wires stores; the surfaces crate is store-agnostic.
- **Checkpoint atomicity (D-F3.8).** Stage 4's unit test injects
  a panic between the `run_checkpoints` insert and the commit and
  asserts the prior checkpoint is still the latest visible state.
  Stage 6's crash-and-resume smoke is the end-to-end form.
- **Bounded `run_checkpoints` growth (D-F3.9).** Stage 4's unit
  test ticks 200 checkpoints with `retention=100`, asserts the
  table has exactly 100 rows for the run with the lowest `seq`
  equal to `101`. Stage 6's soak-style test ticks 10000 times and
  asserts table-size growth stays bounded.
- **Backpressure non-blocking (D-F3.10).** Stage 9's lagging-
  consumer smoke row asserts the producer's per-tick latency
  histogram (captured via tracing) stays below 5ms even when the
  consumer sleeps 50ms per event. A producer that blocks on
  broadcast send violates the decision.
- **Degraded-mode recovery (D-F3.11).** Stage 6's backend-outage
  sub-case asserts the engine transitions Healthy → Degraded →
  Healthy across a 10-second injected outage; the in-flight run
  completes; no new runs are accepted during Degraded; queue
  drains in `(run_id, seq)` order on recovery.
- **At-least-once + dedup (D-F3.12).** Stage 9's
  `flow_as_service` re-delivery sub-case asserts exactly one
  `runs` row exists for two `EventSink::recv` calls with the
  same dedup key.
- **Tick-counter monotonicity.** Stage 6's soak-style test
  asserts `Propagator::current_tick()` returns a strictly
  increasing `u64` across 10000 ticks. Compile-time check via
  `const _: () = assert!(std::mem::size_of::<TickCounter>() ==
  8);` in the propagator module.
- **No tokio task leaks.** Stage 6 + stage 7 unit tests assert
  the `flow_as_service.invoke` and `flow_as_tool.call` tracing-
  span open/close counts balance across a full
  start-process-stop cycle.
- **WAL pragmas applied.** Stage 4's pool-init test asserts
  `PRAGMA journal_mode` returns `wal` and `PRAGMA synchronous`
  returns `1` (NORMAL) on a fresh connection.
- **Forward-only migrations (D-F3 durability hardening
  block).** Stage 4's migration file carries a header comment
  naming the forward-only convention; the convention is checked
  for new Phase 3+ migrations as part of stage 10 verify (a
  comment grep, not an automated test — convention enforcement
  in code review).

## Deliverables

- `crates/starter-flow-spi/src/flow.rs` (+ possibly `session.rs`)
  populated with the three trait method shapes from D-F3.1 plus
  the additive `FlowEvent::CheckpointFailed` /
  `FlowEvent::DedupShortCircuit` variants, the
  `EngineHealth { Healthy, Degraded }` enum, the `EngineError::
  BackendUnavailable` variant, the `RunMetrics` struct with the
  `subscriber_lagged_count` + `degraded_dropped_count` counters,
  the `RunOpts.checkpoint_retention` + `event_broadcast_capacity`
  + `degraded_queue_capacity` fields, and the additive
  `EventSink::dedup_key()` default-`None` method; baseline
  regenerated in same commit.
- `crates/starter-store-sqlite/src/flow/` populated with three
  SQLite store impls under a new `flow` cargo feature; migrations
  in `crates/starter-store-sqlite/migrations/flow/` with WAL
  pragmas in the pool-init path; checkpoint writes wrapped in
  `BEGIN IMMEDIATE` transactions with in-tx pruning; unit tests
  pass under `cargo test -p starter-store-sqlite --features flow`
  and include the atomicity + retention + WAL-pragma cases.
- Engine checkpoint wiring + resume-from-checkpoint path +
  retry-with-backoff + `Degraded` mode + `RunMetrics` accessor in
  `crates/starter-flow/src/`; unit tests assert R2 chokepoint
  integrity through resume, monotonic tick counter, and
  evict-oldest broadcast semantics under producer-faster-than-
  consumer load.
- `crates/starter-flow-surfaces/src/lib.rs`'s `FlowAsTool` body
  populated; unit tests pass including the cancel-within-200ms
  case and the no-task-leak case.
- `crates/starter-flow-surfaces/src/lib.rs`'s `FlowAsService`
  body populated with dedup-key resolution; unit tests pass
  including the re-delivery sub-case and the no-task-leak case.
- `crates/smoke-tests/tests/flow_via_mcp.rs`,
  `crates/smoke-tests/tests/flow_as_service.rs` (covering
  dedup re-delivery), the four-transport-stream-with-FlowEvent
  extension (or standalone smoke if the four-transport file
  doesn't exist) covering lagging-consumer backpressure, and
  `crates/smoke-tests/tests/flow_crash_and_resume.rs` covering
  the SIGKILL-mid-tick + backend-outage durability invariants
  all pass under `cargo test -p starter-smoke-tests`.
- `cargo build --workspace --all-features` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo fmt --check` green.
- `cargo tree -p starter-flow --edges normal | grep adk-rust` and
  `cargo tree -p starter-flow-nodes --edges normal | grep
  adk-rust` return empty.
- `cargo tree -p starter-flow-spi --edges normal` matches the
  regenerated `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
  byte-for-byte.
- None of the four flow crates path-dep onto `starter-mcp` /
  `starter-server` / `starter-cli` (verified by the existing
  dep-tree gates test).

## Open questions (resolve in stage 1)

1. **`SessionStore` location.** Bias: lives in the same
   `starter-flow-spi::flow` module alongside `FlowStore` /
   `RunStore` (smaller module surface to track). If a future
   non-flow consumer needs sessions (likely: a generic auth
   middleware), the trait moves to `starter-spi` at that point.
2. **Checkpoint `seq` source.** Bias: the propagator's own tick
   counter, exposed via a private `Propagator::current_tick()`
   accessor. The counter is monotonic per run and the resume path
   uses `MAX(seq)` to find the latest checkpoint.
3. **`FlowAsService` default principal.** Bias: an
   `Option<Principal>` field on `FlowAsService` carrying the
   service-account principal used when an inbound event carries no
   principal. If `None` and the event carries no principal, the
   service errors the invocation rather than running anonymously.
4. **Four-transport smoke extension vs standalone.** Bias: extend
   the existing file if found; create a standalone
   `flow_event_stream_over_four_transports.rs` if not. Stage 1
   inspects `crates/smoke-tests/tests/` and locks the call.
5. **`run_opts_json` schema in the `runs` table.** Bias: the
   `RunOpts` struct from D1a (`max_propagation_hops`,
   `idempotent_short_circuit`) extended additively with the
   Phase 3 durability fields (`checkpoint_retention`,
   `event_broadcast_capacity`, `degraded_queue_capacity`)
   serialized as-is. Additive future fields are absorbed by
   `#[non_exhaustive]` + a tolerant deserializer.
6. **Soak test tick count.** Bias: 10000 ticks under the in-
   memory store, runtime budget ~10s on CI. Higher tick counts
   (1M for a true long-uptime soak) belong in a nightly CI job,
   not in the per-PR test suite. Stage 1 confirms 10000 is
   sufficient to catch unbounded growth + monotonicity
   regressions without inflating PR runtime.
7. **`EngineHealth` exposure surface.** Bias: a new
   `Engine::health() -> EngineHealth` accessor (sync, lock-free
   read of an `AtomicU8` backing). A future job may add a
   periodic `FlowEvent::HealthChanged` emission on the
   engine-level (not per-run) event bus once that bus exists
   (Phase 7 owns engine-level events). For Phase 3 a pull-based
   accessor is sufficient and avoids inventing a new event
   shape.
8. **Backend retry backoff source.** Bias: hard-coded in the
   engine at the values named in D-F3.11 (50/100/200/400/800ms
   capped at 5 attempts). A future job may make these
   configurable via `RunOpts.checkpoint_backoff` if a consumer
   surfaces a real need; until then a single tuning point is
   simpler.

## Stage 1 — decision lock (prose-only, no code)

Stage 1 verifies the drafted Decisions block above against (a) the
source-of-truth flow SCOPE at
[`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
§"Phase 3 — Persistence + surface wrappers" plus the R6 / R8 / R9
rule blocks, and (b) the actual Phase 2 substrate in the workspace
(merged via `starter-flow-engine-finish`, PR #6). Each D-F3 ID
below restates the lock plus any reality-check addendum the
verification surfaced. The Decisions block above is the canonical
form; this section is the verification record stages 3–10 binds
to.

### Decision lock — D-F3.1 through D-F3.12 (canonical)

- **D-F3.1 — locked.** SPI trait method shapes per the Decisions
  block above. `RunStore` gains
  `find_by_dedup_key(service_name, dedup_key) -> Option<RunId>`
  (named in `template.yaml` stage 3, missing from the prose
  Decisions block above — counted as locked here so the runner
  doesn't drop it). All methods `async`, `&self`,
  `Result<T, FlowError>`. `FlowError` extended additively with
  `NotFound { kind: &'static str, id: String }`. `#[non_exhaustive]`
  on every new public enum + config struct.
- **D-F3.2 — locked.** Per-tick checkpoint cadence, not per
  `write_slot`. Propagator's existing tick counter is `seq`.
- **D-F3.3 — locked.** Three SQLite impls under
  `crates/starter-store-sqlite/src/flow/` behind a default-off
  `flow` feature; migrations in
  `crates/starter-store-sqlite/migrations/flow/`.
  Reality check: the current crate layout has
  [`crates/starter-store-sqlite/src/{lib.rs, migrate/, paging/, pool/, testing/}`](../../../crates/starter-store-sqlite/src/)
  and [`migrations/starter/`](../../../crates/starter-store-sqlite/migrations/)
  — Phase 3 adds `src/flow/` and `migrations/flow/` as siblings,
  matching the existing naming convention.
- **D-F3.4 — locked.** Explicit schemas at
  `FlowAsTool::new(…)`, not derived from the flow revision body.
- **D-F3.5 — locked.** `FlowAsService` subscribes on
  `Service::start`, drains and unsubscribes on `Service::stop`.
- **D-F3.6 — locked.** Four Phase 3 smokes live under
  `crates/smoke-tests/tests/` per the D1d revisit-trigger from
  the Phase 2 SCOPE. Reality check: the directory currently
  holds five workspace-level invariant smokes
  (`smoke_1_no_dep_leakage.rs` through
  `smoke_5_shutdown_actually_shuts_down.rs`); the four Phase 3
  files land as siblings using a `flow_*.rs` prefix to keep
  ordering visible in `ls`.
- **D-F3.7 — locked.** Phase 1 baseline file at
  [`DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`](../../../DOCS/flow/scope/starter-flow-spi-deps.baseline.txt)
  regenerated once in stage 3's single commit, even if the
  fleshout adds no new deps. Stages 4–10 producing a baseline
  diff is a stage-fail.
- **D-F3.8 — locked.** `BEGIN IMMEDIATE` per-tick atomic-tx
  checkpoint; pool-init pragmas
  `journal_mode=WAL, synchronous=NORMAL, busy_timeout=5000,
  foreign_keys=ON` applied by extending (not replacing) the
  existing [`crates/starter-store-sqlite/src/pool/`](../../../crates/starter-store-sqlite/src/pool/)
  connection-init path.
- **D-F3.9 — locked.** In-tx pruning at end of each
  `SqliteRunStore::checkpoint`. Default
  `RunOpts.checkpoint_retention = Bounded(100)` per open run;
  one final row per finished run preserved.
- **D-F3.10 — locked.** `tokio::sync::broadcast` capacity from
  `RunOpts.event_broadcast_capacity` (default 1024); producers
  never block; slow consumers see `RecvError::Lagged(n)` on
  next `recv`; engine-owned per-run subscriber increments
  `RunMetrics.subscriber_lagged_count`; exposed via
  `Engine::run_metrics(run_id)`.
- **D-F3.11 — locked.** Retry-with-backoff 50→100→200→400→800ms
  capped at 5 attempts; after the 5th the engine transitions to
  `EngineHealth::Degraded` (per-engine atomic state, not
  per-run); in-flight runs keep serving from in-memory state
  with checkpoint queue bounded by
  `RunOpts.degraded_queue_capacity` (default 1024, evict-oldest
  with `RunMetrics.degraded_dropped_count` increment);
  `Engine::start` returns `EngineError::BackendUnavailable`
  while `Degraded`; successful checkpoint drains the queue in
  `(run_id, seq)` order and clears `Degraded`. Reality-check
  addendum: SCOPE prose elsewhere in this file says the engine
  "keeps producing `SlotChanged` events for in-flight runs"
  during the outage. The per-run `FlowEvent` stream variant in
  [`crates/starter-flow-spi/src/flow.rs`](../../../crates/starter-flow-spi/src/flow.rs)
  is `NodeEmitted` (per-graph `GraphEvent::SlotChanged` is the
  internal name on the graph-level bus and is not re-exported
  onto the per-run stream). The Stage-9 smoke and the Stage-6
  durability test assert on `FlowEvent::NodeEmitted` for the
  "in-flight runs keep emitting" invariant; the SCOPE prose
  reference to `SlotChanged` is a doc-only carry-over, not a
  new variant to add.
- **D-F3.12 — locked.** Per-event dedup key via
  `EventSink::dedup_key()` (additive optional method with
  default `None` impl on the existing trait in
  [`crates/starter-spi/src/service/sink.rs`](../../../crates/starter-spi/src/service/sink.rs))
  with `blake3((service_name, event_id, payload_bytes))`
  fallback; `UNIQUE (service_name, dedup_key)` partial index on
  the `runs` table; re-delivered events short-circuit via
  `RunStore::find_by_dedup_key` and emit
  `FlowEvent::DedupShortCircuit`.

### Open-question resolutions (Q1–Q8)

- **Q1 — locked: bias accepted.** `SessionStore` lives in
  `starter-flow-spi::flow` (single-module surface). Move to
  `starter-spi` if a non-flow consumer surfaces a need.
- **Q2 — locked: bias accepted.** Checkpoint `seq` is the
  propagator's existing tick counter via a private
  `Propagator::current_tick()` accessor; resume uses
  `MAX(seq)` at the store level.
- **Q3 — locked: bias accepted.** `FlowAsService.default_principal:
  Option<Principal>`. `None` + event-with-no-principal is a typed
  invocation error; the service continues processing subsequent
  events.
- **Q4 — locked: STANDALONE.** Inspection of
  [`crates/smoke-tests/tests/`](../../../crates/smoke-tests/)
  finds five files, none of which is a four-transport stream
  smoke (the existing five are workspace-level invariants:
  dep-leakage, special-case wiring, config-guarded
  construction, secrets backend swap, shutdown actually shuts
  down). Stage 9 lands a standalone
  `crates/smoke-tests/tests/flow_event_stream_over_four_transports.rs`
  covering REST SSE, MCP streaming, gRPC streaming, and
  JSON-RPC stdio against a `FlowEvent` source, with one
  lagging-consumer sub-row per transport asserting non-zero
  `RunMetrics.subscriber_lagged_count` while the run still
  finishes successfully (D-F3.10 backpressure invariant). The
  four-transport extension form named in the SCOPE Smoke block
  is moot — there is nothing to extend.
- **Q5 — locked: bias accepted with addendum.** `runs.run_opts_json`
  serializes the full `RunOpts` struct. Reality-check addendum:
  the existing engine uses
  [`PropagatorConfig { max_propagation_hops }`](../../../crates/starter-flow/src/propagator.rs)
  (no `RunOpts` type exists yet in Phase 2). Stage 3 introduces
  `RunOpts` net-new in `starter-flow-spi` carrying
  `max_propagation_hops`, `idempotent_short_circuit`,
  `checkpoint_retention`, `event_broadcast_capacity`,
  `degraded_queue_capacity` (all `#[non_exhaustive]`-absorbed
  for future fields). Stage 5's `Engine::with_run_store(…)`
  builder hook accepts a `RunOpts` per run and projects
  `max_propagation_hops` into the existing
  `PropagatorConfig` — no rename or refactor of the propagator
  config type (out-of-scope per WORKFLOW anti-pattern
  "refactoring the engine").
- **Q6 — locked: 10000 ticks accepted.** Soak case in
  `flow_crash_and_resume.rs` sub-case (c) ticks 10000 times
  under the in-memory store, ~10s budget on CI. A 1M-tick
  long-uptime soak lands in a nightly CI job in a follow-up
  (out of scope; tracked as a Phase-3-follow-up note in
  `handover.md`).
- **Q7 — locked: bias accepted.** `Engine::health() ->
  EngineHealth` sync accessor backed by an `AtomicU8`. No
  `FlowEvent::HealthChanged` shape this job — engine-level
  events are a Phase 7 concern.
- **Q8 — locked: bias accepted.** Backoff schedule hard-coded
  in the engine at 50/100/200/400/800ms, 5 attempts.
  `RunOpts.checkpoint_backoff` is a follow-up if a consumer
  surfaces a real need.

### Source-of-truth alignment check

The job SCOPE.md above is consistent with
[`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
§"Phase 3 — Persistence + surface wrappers" plus R6 / R8 / R9.
The doc's Phase 3 block names: `FlowStore` + `RunStore` impls in
`starter-store-sqlite` behind a `flow` feature; run checkpointing
on slot writes; resume from checkpoint after a process restart;
`starter-flow-surfaces::{FlowAsTool, FlowAsService}`; the three
SCOPE smokes (MCP-invoked flow, flow-as-Service, four-transport
extended with a `FlowEvent` source). The job SCOPE.md
**additively** lands the 24/7 durability hardening on top
(atomic-tx checkpoint, bounded checkpoint history, backend-
failure `Degraded` posture, backpressure semantics, at-least-once
+ dedup, the crash-and-resume smoke). The doc does not preclude
any of those additions; they are the load-bearing difference
between "Phase 3 compiles" and "Phase 3 deploys". Per the
opening preamble of this file, when the job SCOPE.md disagrees
with the doc, the doc wins. No disagreement found in stage 1.

### Phase 2 substrate verification

Verified in stage 1 against the live workspace:

- [`crates/starter-flow-spi/src/flow.rs`](../../../crates/starter-flow-spi/src/flow.rs)
  declares `FlowStore` and `RunStore` as empty trait seams
  (`pub trait FlowStore: Send + Sync + 'static {}`) — matches
  the Phase 1 posture the SCOPE assumes. No `SessionStore`
  trait yet — Phase 3 adds it.
- `FlowEvent` variants present in Phase 2:
  `RunStarted, NodeStarted, NodeEmitted, NodeFailed,
  RunCompleted, RunFailed, RunCancelled` — all
  `#[non_exhaustive]`-friendly under the existing
  `#[non_exhaustive]` attribute. Stage 3's
  `CheckpointFailed` and `DedupShortCircuit` additions are
  source-compatible.
- [`crates/starter-flow/src/propagator.rs`](../../../crates/starter-flow/src/propagator.rs)
  carries `PropagatorConfig { max_propagation_hops: u64 }`
  (default 1000) — the tick counter is the source for the
  `(run_id, seq)` key per D-F3.2 + Q2.
- [`crates/starter-flow/src/state.rs`](../../../crates/starter-flow/src/state.rs)
  + [`run.rs`](../../../crates/starter-flow/src/run.rs)
  carry `events_tx: broadcast::Sender<FlowEvent>` per-run —
  D-F3.10 backpressure semantics layer on top by constructing
  the sender with `RunOpts.event_broadcast_capacity` (default
  1024 replaces the current hardcoded capacity).
- [`crates/starter-spi/src/service/sink.rs`](../../../crates/starter-spi/src/service/sink.rs)
  defines `trait EventSink { async fn emit(…) -> SinkResult<()>; }`.
  D-F3.12's `dedup_key(&self, _event: &Event) -> Option<String>`
  addition needs a concrete `Event` type — Phase 2 has none.
  Stage 3 introduces a minimal `Event` envelope type on the
  `EventSink` trait or, simpler, places `dedup_key` on the
  emitted payload by changing the method signature to
  `dedup_key(&self, kind: &str, payload: &Value) -> Option<String>`
  with a default `None` impl. Final signature picked at stage 3
  implementation; both shapes are source-compatible additive
  changes that leave the existing `emit(…)` method untouched.
  Flagged here as a Stage-3 implementation choice, not a
  stage-1 lock — both candidates honour the D-F3.12 contract.
- [`crates/starter-store-sqlite/src/{lib.rs, pool/, migrate/}`](../../../crates/starter-store-sqlite/src/)
  and [`migrations/starter/`](../../../crates/starter-store-sqlite/migrations/)
  match the SCOPE's assumed crate layout — Phase 3's
  `src/flow/` and `migrations/flow/` slot in cleanly.
- [`crates/smoke-tests/tests/`](../../../crates/smoke-tests/tests/)
  holds five workspace-level invariant smokes; no
  four-transport stream smoke exists (Q4 → standalone).

### Decisions block the runner must bind to

Stages 3 through 10 bind to the locked D-F3.1 through D-F3.12
decisions plus the Q1–Q8 resolutions above. The single
implementation-choice flag carried into stage 3 is the precise
shape of `EventSink::dedup_key` (`(&self, &Event)` vs
`(&self, kind: &str, payload: &Value)`); either honours D-F3.12,
both are additive and source-compatible. No other open items.

