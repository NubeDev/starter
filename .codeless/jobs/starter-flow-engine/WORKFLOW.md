# Workflow — starter-flow-engine

How to drive this job. Shape: lock D1 + the in-memory substrate
decisions at the entry gate, then land the engine substrate
(GraphStore → propagator → registries → engine → run) bottom-up,
then land the two Phase-2 node bodies on top, then prove the two
SCOPE Smoke tests pass green.

## Sequencing

- **Stage 1 is prose-only.** Lock D1 plus the three sub-decisions
  (cycle-budget defaults, in-memory store substrate, FlowEvent
  stream cardinality) in [SCOPE.md](./SCOPE.md), record under
  "Decisions". Commit; no code.
- **Stage 2 is the entry-gate REVIEW.** Do not advance until the
  user signs off on D1's lock toward (b) `starter-flow-node-loop`
  and the substrate choices. Phase 4 will read this lock when it
  ships; getting it wrong here forces a rebuild later.
- **Stages 3 → 7 land the engine substrate bottom-up.** Strict
  order: GraphStore (3) → propagator (4) → registries (5) →
  engine state machine (6) → run lifecycle + RunState (7). Each
  depends on the previous; the order is not negotiable.
- **Stage 8 is the engine-side REVIEW.** Confirm the engine API
  composes end-to-end on the two-node smoke (a `transform` node
  feeding a downstream node) before locking it for the node
  bodies. The next two stages bind to whatever API ships out of
  this gate.
- **Stages 9 and 10 land the two Phase-2 node bodies.**
  `transform` first (pure function — simpler shape, exercises the
  GraphStore chokepoint), then `tool-call` (exercises the
  ToolRegistry seam + Cancel propagation).
- **Stage 11 is the Phase 2 exit smoke.** "One write chokepoint"
  and "Engine is reader of policies" — both word-for-word with
  the flow SCOPE Smoke-tests block. If either fails, the
  milestone has not landed; fix the cause, do not advance.
- **Stage 12 is workspace-wide verify + dep-tree gates.** The
  dep-tree gates are the cheapest signal that nothing pulled
  `adk-rust` in by accident, and that the Phase 1 `-spi` baseline
  did not regress under Phase 2's engine landing.

## Per-stage discipline

- **Before any code change in a stage:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the stage
    touches. R2 (chokepoint + replay semantics + idempotent
    short-circuit), R3 (no policy-name match arms), R5 (`&self`),
    R7 (skill seam wired, body not landed), R8 (Tool ≠ Node),
    R10 (reserved prefix enforced), R12 (state machine), R13
    (Cancel + broadcast streams) are the load-bearing rules for
    this phase.
  - Re-read the SCOPE section that names the module you are
    creating. Every new module's head doc comment cites the
    SCOPE section by name so a future reader does not re-derive
    the design from code.
- **Touch only what the stage names.** No drive-by refactors.
  The Phase-1 scaffold already touched many crates; if a sibling
  crate needs a fix, surface it as an issue and a separate small
  PR — do not enlarge this job mid-stage.
- **Verify before commit:**
  - **Rust per-stage:** `cargo check -p <touched crate>`, then
    `cargo test -p <touched crate>`, then
    `cargo clippy --workspace --all-targets -- -D warnings`,
    then `cargo fmt --check`.
  - **Dep-tree per Rust stage:** re-run
    `cargo tree -p starter-flow --edges normal` and
    `cargo tree -p starter-flow-nodes --edges normal`. Either
    showing `adk-rust` is a stage-fail; revert and find what
    pulled it in.
  - **Dep-tree baseline per Rust stage:** re-run
    `cargo tree -p starter-flow-spi --edges normal` and
    `diff` against `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`.
    A diff means either the baseline updates (separate PR, not
    here — this job does not modify `-spi`) or the change
    accidentally added a dep to `-spi` (revert).
- **One logical batch per commit.** The closing trio at the end
  of every stage is the heartbeat the UI watches.

## REVIEW gates

Two:

- **After stage 1 (Phase 2 entry gate).** D1 plus the three
  sub-decisions. Four small questions; locking them down first
  is cheap; getting D1 wrong forces a Phase-4 rebuild. The
  REVIEW exists because the rest of the job binds to the
  decisions.
- **After stage 7 (engine substrate complete).** Compose the
  five engine modules end-to-end via a two-node smoke (a
  `transform` placeholder feeding a downstream node); confirm
  the API has no hot-spots regret. Stages 9 and 10 bind to
  whatever API ships out of this gate. The smoke at stage 11 is
  word-for-word with the SCOPE; this gate is the *internal* one
  that lets the node-kind authors not chase a moving target.

Stage 11 (the Phase 2 SCOPE Smoke) is itself a verification
stage — the test pass is the merge gate, not a third REVIEW.

Write a one-line summary into `handover.md` at every gate. Do
not proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled for D1 + 3 sub-decisions (D-S2.1 through D-S2.4); no code changed; commit message references each decision by ID. |
| 3 | `crates/starter-flow/src/graph.rs` populated; `impl GraphStore for InMemoryGraphStore` complete; write_slot enforces R2 replay + idempotent short-circuit + emits a tracing span carrying both flags; unit tests pass for single-writer, replay-suppress, idempotent short-circuit, force-defeat, subscribe-from-now semantics; `cargo test -p starter-flow` green; clippy + fmt green. |
| 4 | `crates/starter-flow/src/propagator.rs` populated; synchronous tokio loop subscribed to `SlotChanged`; cycle-budget counter wired; Cancel-token integration wired; unit tests pass for linear three-node propagation, one-cycle idempotent termination, forced-no-shortcut cycle terminates on `max_propagation_hops`, mid-run Cancel stops scheduling; clippy + fmt green. |
| 5 | `crates/starter-flow/src/registry.rs` populated; `NodeKindRegistry` + `FlowRegistry` both `tokio::sync::RwLock`-protected; reserved `starter.flow.*` prefix enforced only on host-internal `register_builtin` path; unit tests pass for refuse-non-host-on-reserved, duplicate-refuse, lookup-after-register, lookup-after-deregister, FlowRegistry multi-revision lookup; clippy + fmt green. |
| 6 | `crates/starter-flow/src/engine.rs` populated; type-state EngineState enum + transition matrix per R12; `Engine::start` / `Engine::pause` / `Engine::resume` / `Engine::stop` all wired; stop walks `IsWritable` and writes safe-state per R3; unit tests pass for legal-transition matrix, illegal-transition typed-error, safe-state-on-stop with a fake writable kind; clippy + fmt green. |
| 7 | `crates/starter-flow/src/run.rs` + `crates/starter-flow/src/state.rs` populated; `RunState` engine-typed; `FlowRunner::start` calls `SkillSelector::select` exactly once per outer run and threads `Arc<SkillSelection>` through to `NodeBehavior::invoke`; `RunStore` trait + in-memory `Vec<RunState>` impl for tests; unit tests pass for happy-path `FlowEvent` emission sequence, mid-run Cancel emits `RunCancelled` within bounded time, cycle-exhausted emits `RunFailed { reason: cycle-budget-exhausted }`, skill-selector called exactly once; clippy + fmt green. |
| 9 | `crates/starter-flow-nodes/src/transform.rs` populated; `NodeBehavior::invoke` reads input slot, applies the Phase 2 transform substrate locked in stage 1 sub-decision, writes through `GraphStore::write_slot` (never bypassing R2); `&self` per R5; unit tests pass for identity, arithmetic, panic-surfacing-as-NodeFailed; `cargo check -p starter-flow-nodes --features transform` green; clippy + fmt green. |
| 10 | `crates/starter-flow-nodes/src/tool_call.rs` populated; looks up the `Tool` in the host-provided `ToolRegistry` per R8; invokes with the run's Principal + Cancel + EventSink; writes result through the chokepoint; `&self`; unit tests use a `MockTool` and pass for happy-path tool dispatch, tool-typed-error surfaces as `NodeFailed`, mid-call Cancel propagates; `cargo check -p starter-flow-nodes --features tool-call` green; clippy + fmt green. |
| 11 | `crates/smoke-tests/` contains "one-write-chokepoint" and "engine-reader-of-policies" tests; both green under `cargo test -p smoke-tests --features flow`; the R3 grep-test (no match arms on policy slot names in `crates/starter-flow/src`) is included in the same suite. |
| 12 | `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo fmt --check` green; `cargo tree -p starter-flow-spi --edges normal` matches `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` byte-for-byte; `cargo tree -p starter-flow --edges normal` and `cargo tree -p starter-flow-nodes --edges normal` contain zero `adk-rust` entries; no path-dep from any of the four flow crates to `starter-mcp` / `starter-server` / `starter-cli` (Phase 3 surfaces wiring, not here). |

## Anti-patterns

- **A second write path.** The propagator does not call into
  storage directly; it calls `GraphStore::write_slot`. The
  engine's safe-state pass calls `write_slot`. The replay path
  calls `write_slot`. Every external API call eventually calls
  `write_slot`. If a stage adds a second function on `GraphStore`
  that mutates state, R2 has slipped. **The R2 smoke in stage 11
  catches this; the WORKFLOW exists so it never gets that far.**
- **A `match policy.session_policy { … }` arm anywhere in
  `starter-flow`.** R3 says the engine reads policies as data,
  not as types. Same for `on_failure`, `cost_cap`, `safe_state`,
  `trigger`, `auth`, `timeout`. The stage-11 grep-test catches
  this; the WORKFLOW exists so it never gets there.
- **Importing `adk-rust`.** D1 is locked toward (b); this
  workspace stays adk-rust-free through Phase 2. Any stage that
  finds itself wanting `use adk_rust::…` has misread D-S2.1; the
  fix is in `starter-flow-node-loop` (Phase 4), not here.
- **`&mut self` on `NodeBehavior::invoke`.** R5. Per-instance
  state lives in slots or in the run/session store. A stage
  that thinks it needs `&mut self` is conflating per-node config
  with per-instance state.
- **Landing the `ai-agent` body "because the seam is here."**
  Phase 4. Stage 7 wires the outer-run skill-selection seam;
  Phase 2 leaves the kind module a `KIND_ID` constant exactly as
  Phase 1 left it.
- **A `Tool` impl on a node.** R8. The mapping goes both
  directions through wrapper kinds (`tool-call` wraps Tools;
  `FlowAsTool` wraps flows as Tools — Phase 3). The two traits
  stay separate.
- **Engine-level event bus.** R13 says streaming uses the same
  shape `starter-ai::OnEvent` and `starter_spi`'s event channels
  use. `FlowEvent` is per-run `broadcast`, not engine-global.
- **`unsafe`.** `starter-flow` keeps `#![forbid(unsafe_code)]`
  from Phase 1. Anywhere unsafe seems necessary, the design is
  wrong.
- **Touching `starter-flow-spi`.** Phase 1 froze the contracts.
  A real gap is an issue + a separate PR, not a stage in this
  job. If a stage finds itself reaching for `-spi`, stop and
  open the issue first.
- **Touching `starter-flow-surfaces`.** Phase 3. Empty structs
  stay empty until then.
- **Importing `starter-mcp` / `starter-server` / `starter-cli`
  from any of the four flow crates.** R8 + R9 say surfaces wire
  the other way (the existing crates consume `FlowAsTool` /
  `FlowAsService` when Phase 3 ships them). A flow crate
  depending on a surface crate is the dependency inverted.
  Stage 12 cargo-tree check enforces this.

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
   the job's branch (`codeless/starter-flow-engine`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
