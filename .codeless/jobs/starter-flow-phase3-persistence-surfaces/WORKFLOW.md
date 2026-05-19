# Workflow — starter-flow-phase3-persistence-surfaces

How to drive Phase 3 with 24/7 supervisory-runtime rigor (Niagara-
style: years of uptime, crash-safe, backend-disruption-tolerant,
bounded resource growth). Shape: lock the twelve small decisions at
the entry gate, then land the SPI fleshout, then the SQLite store
impls, then the engine checkpoint wiring, then the durability
hardening, then the two surface wrappers, then the four SCOPE smoke
tests (including the crash-and-resume durability proof), then the
workspace-wide verify that re-confirms the existing Phase 1/2 dep-
tree gates.

This is a **phase-completion job, not a phase-fragment.** The
merged Phase 2 sibling [`starter-flow-engine-finish`](../starter-flow-engine-finish/)
landed everything Phase 2 promised; this job lands everything
Phase 3 promises **and the durability hardening that makes the
result fit for continuous operation**. Keep the changes scoped to
the deliverables named in [SCOPE.md](./SCOPE.md).

## Sequencing

- **Stage 1 is prose-only.** Lock D-F3.1 through D-F3.12 in
  [SCOPE.md](./SCOPE.md), record under "Decisions". Commit; no
  code. Stage 1 also inspects `crates/smoke-tests/tests/` to
  resolve open question 4 (four-transport smoke: extend or
  standalone) and records the call.
- **Stage 2 is the entry-gate REVIEW.** Do not advance until the
  user signs off — particularly on D-F3.1 (SPI trait shapes),
  D-F3.2 (per-tick checkpoint cadence), D-F3.6 (smoke location),
  D-F3.8 (atomic checkpoint writes + WAL pragmas), D-F3.11
  (Degraded-mode backend-failure posture), and D-F3.12 (at-least-
  once + dedup) — these six cascade into stages 3–9 and getting
  them wrong means re-do.
- **Stage 3 lands the SPI trait fleshout in one commit.** Trait
  method shapes per D-F3.1 plus the additive `FlowEvent` /
  `EngineHealth` / `EngineError` / `RunMetrics` / `RunOpts` /
  `EventSink::dedup_key()` extensions per the durability
  decisions; baseline regenerated in the same commit per D-F3.7.
  No store impls in this stage; the surfaces crate and the
  engine see the new trait methods on the next build but don't
  yet consume them.
- **Stage 4 lands three SQLite store impls in one commit.**
  Migrations + `schema` module + `SqliteFlowStore` +
  `SqliteRunStore` (with `BEGIN IMMEDIATE` atomic-tx checkpoint
  + in-tx pruning per D-F3.8 + D-F3.9 + WAL pragmas in the
  pool-init path) + `SqliteSessionStore` (with the
  `UNIQUE (service_name, dedup_key)` index for D-F3.12) + their
  unit tests. Behind the new `flow` feature on
  `starter-store-sqlite`. Default-off per D-F3.3.
- **Stage 5 wires checkpointing into the engine.**
  `Engine::with_run_store(…)` builder hook + per-tick
  `Propagator::maybe_checkpoint(…)` + resume-from-checkpoint
  path on engine start. Unit test asserts R2 chokepoint
  integrity through resume. The existing Phase 2 smokes must
  stay green; run them locally before commit.
- **Stage 6 lands the durability hardening.** Five things, all
  load-bearing for 24/7 operation: (a) retry-with-backoff on
  `RunStore::checkpoint` errors per D-F3.11 (50→100→200→400→
  800ms, 5 attempts, then `Degraded`); (b) the
  `EngineHealth { Healthy, Degraded }` state + `Engine::health()`
  accessor + the in-memory queue with `RunOpts.
  degraded_queue_capacity` cap (default 1024, evict-oldest);
  (c) `Engine::start` returning `EngineError::BackendUnavailable`
  when `Degraded`; (d) the per-run broadcast capacity hook with
  `RunOpts.event_broadcast_capacity` (default 1024) + the
  engine's own `Lagged`-watching subscriber that increments
  `subscriber_lagged_count` on `RunMetrics`; (e) the monotonic
  `u64` tick-counter assertion and the
  `const _: () = assert!(std::mem::size_of::<TickCounter>()
  == 8)` compile-time check. Unit tests cover each invariant
  in isolation; stage 9's crash-and-resume smoke is the
  end-to-end form.
- **Stage 7 lands `FlowAsTool` body.** Fields per D-F3.4
  (explicit schemas at construction). Unit tests cover: a
  wrapped flow surfaces as `Tool::call` that returns the
  flow's terminal output slot value; a flow that errors
  surfaces as a typed `ToolError`; a `Cancel` fired during
  the call propagates into the engine's per-run cancel and
  the run reports `Cancelled` within 200ms; no tokio task
  leaks (span open/close balance).
- **Stage 8 lands `FlowAsService` body.** Lifecycle per
  D-F3.5 (subscribe on `start`, drain on `stop`). Dedup-key
  resolution per D-F3.12 (`EventSink::dedup_key()` first,
  blake3 fallback). Unit tests cover: three events drive
  three runs to `Finished`; `stop` cancels in-flight and
  joins; an event with no principal under a `None` default
  principal surfaces an invocation error; a re-delivered
  event with the same dedup key short-circuits to the prior
  run's outcome and emits `FlowEvent::DedupShortCircuit`; no
  tokio task leaks.
- **Stage 9 lands the four Phase 3 SCOPE smokes.** All files
  live under `crates/smoke-tests/tests/` per D-F3.6.
  WORKFLOW pins ordering: `flow_via_mcp.rs` first (exercises
  `FlowAsTool` + `SqliteRunStore` together), then
  `flow_as_service.rs` (exercises `FlowAsService` +
  `SqliteRunStore` + dedup re-delivery), then the four-
  transport extension or standalone (exercises the
  `FlowEvent` stream across all four transports including
  the lagging-consumer backpressure row), then
  `flow_crash_and_resume.rs` (the 24/7 durability proof:
  SIGKILL-mid-tick resume, 10s backend outage with
  `Degraded` recovery, 10000-tick soak). One commit per
  smoke file; bundling is a WORKFLOW-fail per the SCOPE
  "One logical batch per stage" rule.
- **Stage 10 is workspace-wide verify + dep-tree re-confirm.**
  No code changes; just running the gates and confirming
  green. Specifically the existing
  [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
  test still passes after the stage-3 baseline regeneration.
  If any gate fails, fix the cause and retry — never
  `--force`.

## Per-stage discipline

- **Before any code change in a stage:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the stage
    touches. R2 (chokepoint), R3 (no policy match arms), R5
    (`&self`), R6 (sessions/runs/checkpoints engine-typed), R8
    (Tools-are-one-node-kind), R9 (Flows-as-Services), R10
    (KindId), R13 (streaming + cancel) are the load-bearing rules.
  - Re-read the
    [`starter-flow-engine-finish`](../starter-flow-engine-finish/SCOPE.md)
    sibling SCOPE for the Phase 2 substrate decisions this Phase 3
    job binds to (D-F2F.2 transform substrate, D-F2F.3
    ToolRegistry injection, D-F2F.4 R3 grep allow-list — all stay
    in force).
- **Touch only what the stage names.** The engine substrate is
  stable; touching `engine.rs`, `run.rs`, `propagator.rs`,
  `graph.rs`, `registry.rs` for anything other than the stage 5
  checkpoint wiring is out. The two surface wrappers live in
  `starter-flow-surfaces` only; they do not edit
  `starter-flow-spi` (which stage 3 already finalised) or
  `starter-flow-nodes` (which the Phase 2 sibling finalised).
- **Verify before commit:**
  - **Rust per-stage:** `cargo check -p <touched crate>`, then
    `cargo test -p <touched crate>` (with the appropriate feature
    flag for stage 4: `--features flow`), then
    `cargo clippy --workspace --all-targets -- -D warnings`, then
    `cargo fmt --check`.
  - **Dep-tree per Rust stage:** re-run the
    [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs)
    test. A surprise dep is a stage-fail; revert.
  - **Spi baseline per Rust stage (post-stage-3):** re-run
    `cargo tree -p starter-flow-spi --edges normal` and diff
    against the (now regenerated) baseline. A diff means the
    change accidentally touched a transitive dep of `-spi`;
    revert.
  - **Phase 2 smokes per Rust stage:** stage 5 must additionally
    re-run the three Phase 2 smokes
    ([`smoke_one_write_chokepoint`](../../../crates/starter-flow/tests/smoke_one_write_chokepoint.rs),
    [`smoke_engine_is_reader_of_policies`](../../../crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs),
    [`r3_no_policy_match_arms`](../../../crates/starter-flow/tests/r3_no_policy_match_arms.rs))
    and confirm green. The checkpoint wiring is the change most
    likely to regress them.
- **One logical batch per commit.** The closing trio is the
  heartbeat the UI watches.

## REVIEW gates

One:

- **After stage 1 (Phase 3 boundary + 24/7 durability lock).**
  Twelve decisions — D-F3.1 (SPI trait method shapes), D-F3.2
  (checkpoint cadence), D-F3.3 (feature-gated SQLite module),
  D-F3.4 (explicit `FlowAsTool` schemas), D-F3.5 (subscribe-on-
  start service lifecycle), D-F3.6 (smoke location), D-F3.7
  (baseline regeneration discipline), D-F3.8 (atomic checkpoint
  writes + WAL pragmas), D-F3.9 (bounded checkpoint history),
  D-F3.10 (`FlowEvent` backpressure: evict-oldest), D-F3.11
  (`Degraded` mode backend-failure posture), D-F3.12 (at-least-
  once + dedup). Plus the open-question-4 resolution (four-
  transport smoke: extend or standalone). All thirteen cascade
  into the next eight stages; getting them wrong means a re-do.

Stage 10 is itself a verification stage — the dep-tree gates +
the smoke pass are the merge gate, not a second REVIEW.

Write a one-line summary into `handover.md` at the gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" filled for D-F3.1 through D-F3.12; open-question-4 resolved; no code changed; commit references each decision by ID. |
| 3 | `crates/starter-flow-spi/src/flow.rs` (and possibly `session.rs`) populated with the trait methods named in D-F3.1 plus the additive `FlowEvent::CheckpointFailed` / `FlowEvent::DedupShortCircuit` variants, `EngineHealth`, `EngineError::BackendUnavailable`, `RunMetrics`, the new `RunOpts` durability fields, and `EventSink::dedup_key()` default-`None` accessor; `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` regenerated in the same commit; the existing dep-tree gates test still passes against the new baseline; `cargo test -p starter-flow-spi` green; clippy + fmt green. |
| 4 | `crates/starter-store-sqlite/src/flow/{flow_store,run_store,session_store}.rs` + `crates/starter-store-sqlite/migrations/flow/*.sql` all present under a default-off `flow` feature; `SqliteRunStore::checkpoint` wraps in `BEGIN IMMEDIATE` with in-tx pruning per D-F3.8 + D-F3.9; pool-init applies the WAL pragmas; the `UNIQUE (service_name, dedup_key)` index exists on `runs`; unit tests cover atomicity (panic-between-insert-and-commit leaves prior checkpoint visible), retention (200 ticks → 100 rows), WAL-pragma application, dedup-uniqueness enforcement; `cargo test -p starter-store-sqlite --features flow` green; `cargo test -p starter-store-sqlite` (no flow feature) still green; clippy + fmt green. |
| 5 | `Engine::with_run_store(…)` + `Propagator::maybe_checkpoint(…)` + resume-from-checkpoint path all landed in `crates/starter-flow/src/`; new unit test asserts R2 chokepoint integrity through resume; the three Phase 2 smokes still pass; `cargo test -p starter-flow` green; clippy + fmt green. |
| 6 | Durability hardening landed: retry-with-backoff (50→100→200→400→800ms, 5 attempts) on `RunStore::checkpoint` errors; `EngineHealth { Healthy, Degraded }` state + `Engine::health()` accessor; in-memory checkpoint queue under `RunOpts.degraded_queue_capacity` cap (default 1024, evict-oldest); `Engine::start` returns `EngineError::BackendUnavailable` when `Degraded`; per-run broadcast capacity hook + engine's `Lagged`-watcher incrementing `RunMetrics.subscriber_lagged_count`; monotonic `u64` tick-counter assertion + `const _` size check. Unit tests pass each invariant in isolation. The three Phase 2 smokes still green. `cargo test -p starter-flow` green; clippy + fmt green. |
| 7 | `FlowAsTool` body populated per D-F3.4 in `crates/starter-flow-surfaces/src/lib.rs`; `Tool::call` forwards into the engine; unit tests cover happy-path / error / cancel-within-200ms / no-task-leak; `cargo test -p starter-flow-surfaces` green; clippy + fmt green. |
| 8 | `FlowAsService` body populated per D-F3.5 + D-F3.12 in `crates/starter-flow-surfaces/src/lib.rs`; `Service::start` subscribes; `Service::stop` cancels in-flight and joins; dedup-key resolution `EventSink::dedup_key()` → blake3 fallback; unit tests cover three-event drive / clean stop / missing-principal error / re-delivery short-circuit / no-task-leak; `cargo test -p starter-flow-surfaces` green; clippy + fmt green. |
| 9 | `crates/smoke-tests/tests/flow_via_mcp.rs`, `crates/smoke-tests/tests/flow_as_service.rs` (including dedup re-delivery sub-case), the four-transport-with-FlowEvent file (extension or standalone per stage-1 resolution, including the lagging-consumer backpressure row), and `crates/smoke-tests/tests/flow_crash_and_resume.rs` (SIGKILL-mid-tick resume + 10s backend outage + 10000-tick soak) all present and passing under `cargo test -p starter-smoke-tests`; each smoke fails loudly when its contract is broken (manually break in a scratch branch, confirm detection, revert); clippy + fmt green. |
| 10 | `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo fmt --check` green; the existing [`workspace_dep_tree_gates`](../../../crates/starter-flow/tests/workspace_dep_tree_gates.rs) test passes (the regenerated baseline holds; no `adk-rust` under any flow crate; no flow crate depends on `starter-mcp` / `starter-server` / `starter-cli`); the four Phase 3 smokes from stage 9 pass under `cargo test --workspace`. |

## Anti-patterns

- **Refactoring the engine.** The merged Phase 2 substrate is what
  this Phase 3 job sits on. A stage that finds itself rewriting
  `propagator.rs` or `engine.rs` for "cleanup" has exceeded scope;
  surface the defect as an issue and a separate PR. The legitimate
  edits are exactly the three named in stage 5
  (`Engine::with_run_store`, `Propagator::maybe_checkpoint`,
  resume-on-start).
- **Per-write checkpointing.** D-F3.2 pins per-tick cadence. A
  stage that calls `RunStore::checkpoint` from inside
  `GraphStore::write_slot` has violated the decision and fights
  R2's idempotent-write short-circuit.
- **Coupling `starter-flow-surfaces` to `starter-store-sqlite`.**
  The surfaces crate is store-agnostic; the host wires the store.
  A stage that adds `starter-store-sqlite` to
  `starter-flow-surfaces`'s `Cargo.toml` has broken the dep-graph
  shape Phase 3 promises (and the dep-tree gates test will catch
  it at stage 9).
- **Bundling Phase 3 smokes into the engine-crate tests dir.**
  D-F3.6 pins them to `crates/smoke-tests/`. A stage that thinks
  "smokes for the engine should live in the engine crate" has
  missed the D1d revisit-trigger that fires exactly here.
- **Deriving `FlowAsTool` schemas from the flow revision.**
  D-F3.4 keeps them explicit. Phase 5 (richer typed `transform`)
  is the right time to add a derived constructor; not here.
- **Subscribing to the `EventSink` at construction time.**
  D-F3.5 subscribes on `start`. Construction-time subscriptions
  leak task handles when the host builds N services before
  starting any of them.
- **`&mut self` on the propagator's per-invoke API.** R5. Builder
  hook is the one place a builder-style `Self`-by-value transform
  is acceptable.
- **Touching `starter-flow-spi` after stage 3.** Stage 3 finalises
  the SPI fleshout. Stages 4–9 that touch the SPI crate
  re-introduce baseline drift the dep-tree gates test will catch;
  revert.
- **Pulling `adk-rust` anywhere.** R7 + R1-supersede. Stage 9's
  dep-tree gate catches it.
- **`todo!()` or `unimplemented!()` in any body.** Workspace
  CLAUDE.md no-half-finished-implementation rule applies. If a
  stage cannot complete, mark it `[!]` and halt — do not commit a
  placeholder that compiles but does nothing.
- **`--no-verify` to skip a failing hook.** Never. Fix the cause.
- **Schema changes after stage 4.** The SQLite schema in stage 4
  is the contract for stages 5–9. A migration added in stage 8
  to fix a stage-4 oversight is a stage-4 redo, not an in-place
  patch — revert and re-land stage 4 cleanly.
- **Blocking the producer on a full broadcast channel.** D-F3.10
  pins evict-oldest. A stage that switches to a bounded mpsc
  with `send().await` will deadlock the engine off any slow
  consumer; reject in code review.
- **Crashing the engine on a single `RunStore` failure.** D-F3.11
  pins retry-with-backoff + `Degraded` mode. A stage that
  `panic!`s or `expect()`s on `RunStore::checkpoint` errors
  violates the 24/7 posture; the engine continues serving
  in-flight runs on backend disruption, period.
- **Unbounded `run_checkpoints` growth.** D-F3.9 pins in-tx
  pruning. A stage that "defers pruning to a background sweep"
  ships a different (more complex) design; reject.
- **`SIGKILL`-unsafe checkpoint writes.** D-F3.8 pins
  `BEGIN IMMEDIATE` + WAL. A stage that batches multiple
  checkpoints into a single tx for throughput violates the
  per-tick atomic-checkpoint contract resume relies on.
- **Silent dedup-key collision.** D-F3.12's `UNIQUE` index makes
  this a database error. A stage that catches the constraint
  violation and starts a second run anyway has defeated the
  whole point; the catch must short-circuit to the prior run.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list (per the "Done when"
   table above). Every step must pass. On failure: stop, fix,
   re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the
   active session doc, in the same worktree, so the fresh agent
   that opens the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to
   the job's branch (`codeless/starter-flow-phase3-persistence-surfaces`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
