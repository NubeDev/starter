# Workflow — starter-flow-engine-finish

How to drive this catch-up job. Shape: lock the four small
decisions at the entry gate, then land two node bodies (one
per stage), then the two SCOPE smoke tests + the R3 grep-test,
then the workspace-wide verify that re-confirms the
already-passing dep-tree gates.

This is a **catch-up job, not a phase**. The merged
[`starter-flow-engine`](../starter-flow-engine/) sibling shipped
the engine substrate. This job lands what that job's stages
9–12 were supposed to land but did not. Keep the changes
surgical.

## Sequencing

- **Stage 1 is prose-only.** Lock D-F2F.1 through D-F2F.4 in
  [SCOPE.md](./SCOPE.md), record under "Decisions". Commit; no
  code.
- **Stage 2 is the entry-gate REVIEW.** Do not advance until
  the user signs off — particularly on D-F2F.1 (smoke-test
  location) since that's the design call most likely to surface
  later regret.
- **Stages 3 → 4 land the two node bodies.** `transform` first
  (simpler shape — pure function), then `tool_call` (registry
  lookup + Cancel propagation). Each stage commits its body +
  its unit tests in the same commit. Order is not negotiable
  because stage 4 may need to verify the engine threads
  `ToolRegistry` through to per-invoke context; if it doesn't,
  stage 4 also lands that threading.
- **Stage 5 lands the two SCOPE smokes.** Both files live in
  `crates/starter-flow/tests/`. The two smokes are independent
  of each other and could in principle land in either order,
  but the WORKFLOW pins "One write chokepoint" first because it
  exercises the simpler path (write → propagate → write) before
  the policy-walk smoke exercises engine stop semantics.
- **Stage 6 lands the R3 grep-contract test.** Single file in
  the same tests directory. Independent of the node bodies but
  depends on stage 5 being green (running both R3-related tests
  in the same suite catches conflicts early).
- **Stage 7 is workspace-wide verify + dep-tree re-confirm.**
  No code changes; just running the gates and confirming green.
  If any gate fails, fix the cause and retry — never `--force`.

## Per-stage discipline

- **Before any code change in a stage:**
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the
    stage touches. R2 (chokepoint), R3 (no policy match arms),
    R5 (&self), R8 (Tool ≠ Node), R10 (KindId validation), R12
    (safe_state walk), R13 (Cancel propagation) are the
    load-bearing rules.
  - Re-read the
    [`starter-flow-engine`](../starter-flow-engine/SCOPE.md)
    sibling SCOPE for the substrate decisions this catch-up
    binds to (D-F2F.2 mirrors that job's stage-1 sub-decision
    on transform; D-F2F.3 mirrors stage-1 on ToolRegistry).
- **Touch only what the stage names.** The engine substrate is
  merged; touching `engine.rs`, `run.rs`, `propagator.rs`,
  `graph.rs`, `registry.rs` for anything other than narrow test
  helpers is out. The one exception is stage 4 — if the engine
  does not already thread `ToolRegistry` through to per-invoke
  context, stage 4 lands that small wire-up in the same commit
  as the `tool_call` body, with the wire-up itself being a
  one-method addition, not a refactor.
- **Verify before commit:**
  - **Rust per-stage:** `cargo check -p <touched crate>`, then
    `cargo test -p <touched crate>` (for stage 5 / 6 the touched
    crate is `starter-flow`; for stage 3 / 4 it's
    `starter-flow-nodes`), then
    `cargo clippy --workspace --all-targets -- -D warnings`,
    then `cargo fmt --check`.
  - **Dep-tree per Rust stage:** re-run `cargo tree -p
    starter-flow --edges normal` and `cargo tree -p
    starter-flow-nodes --edges normal`; both must contain no
    `adk-rust`. A surprise dep is a stage-fail; revert.
  - **Spi baseline per Rust stage:** re-run `cargo tree -p
    starter-flow-spi --edges normal` and diff against
    `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`. A
    diff means the change accidentally touched a transitive dep
    of `-spi`; revert.
- **One logical batch per commit.** The closing trio is the
  heartbeat the UI watches.

## REVIEW gates

One:

- **After stage 1 (catch-up boundary).** Four decisions —
  D-F2F.1 (smoke-test location), D-F2F.2 (transform substrate),
  D-F2F.3 (ToolRegistry injection), D-F2F.4 (R3 grep
  allow-list). All four are small but cascade into the next
  four stages; getting them wrong means a re-do.

Stage 7 is itself a verification stage — the dep-tree gates +
the smoke pass are the merge gate, not a second REVIEW.

Write a one-line summary into `handover.md` at the gate. Do
not proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" filled for D-F2F.1 through D-F2F.4; no code changed; commit references each decision by ID. |
| 3 | `crates/starter-flow-nodes/src/transform.rs` populated; `NodeBehavior` impl + `TransformFunctionRegistry` trait + `StaticTransformFunctionRegistry` impl all present; `KIND_ID` constant unchanged; unit tests pass for identity / arithmetic / panic-surfacing / unregistered-fn-id; `cargo test -p starter-flow-nodes --features transform` green; clippy + fmt green. |
| 4 | `crates/starter-flow-nodes/src/tool_call.rs` populated; `NodeBehavior` impl uses `Arc<dyn ToolRegistry>` injected via the per-run context; if needed, the engine's per-invoke threading is wired in the same commit; `KIND_ID` constant unchanged; unit tests pass for happy-path dispatch / tool-error → NodeFailed / Cancel-within-200ms / unknown-tool-id; `cargo test -p starter-flow-nodes --features tool-call` green; clippy + fmt green. |
| 5 | `crates/starter-flow/tests/smoke_one_write_chokepoint.rs` and `crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs` both present; both pass under `cargo test -p starter-flow --test smoke_one_write_chokepoint` and `--test smoke_engine_is_reader_of_policies`; both fail-loudly when the contract is broken (manually break R2 / R3 in a scratch branch, confirm the tests detect it, revert); clippy + fmt green. |
| 6 | `crates/starter-flow/tests/r3_no_policy_match_arms.rs` present; passes against current source; the allow-list contains exactly one entry (the `WritableOutput::safe_state` trait method); the test fails when a contrived `match foo.session_policy { … }` arm is added to a scratch branch in `starter-flow/src/`; clippy + fmt green. |
| 7 | `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo fmt --check` green; `cargo tree -p starter-flow --edges normal` and `cargo tree -p starter-flow-nodes --edges normal` contain zero `adk-rust`; `cargo tree -p starter-flow-spi --edges normal` matches `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` byte-for-byte; none of the four flow crates path-dep onto `starter-mcp` / `starter-server` / `starter-cli`. |

## Anti-patterns

- **Refactoring the engine.** The merged Phase 2 substrate is
  what this catch-up sits on. A stage that finds itself
  rewriting `propagator.rs` or `engine.rs` for "cleanup" has
  exceeded scope; surface the defect as an issue and a separate
  PR.
- **Bundling the two SCOPE smokes into a single file.** The
  flow SCOPE Smoke-tests block names them separately for a
  reason — when one fails, the test runner's output names the
  specific design rule that broke. Bundling defeats the
  diagnostic.
- **Adding the smokes to `crates/smoke-tests/`.** D-F2F.1 keeps
  them in `starter-flow/tests/`. A stage that thinks "smoke
  tests should live in the smoke-tests crate" has missed that
  the existing crate is owned by the tools SCOPE.
- **Pulling `rhai` into `starter-flow-nodes`.** D-F2F.2 keeps
  it out. The Phase 2 smoke ("transform sums two slots") needs
  Rust closures, not scripting. Phase 5 may add `rhai` as a
  separate feature on `starter-flow-nodes`; not here.
- **`&mut self` on `NodeBehavior::invoke`.** R5. Per-instance
  state lives in slots or in the run/session/registry surface.
- **Using a global static for the `ToolRegistry`.** D-F2F.3
  threads it per-run via the engine's construction args.
  Global statics defeat per-flow registry overlays (a future
  decision point).
- **Loosening the R3 grep-test allow-list with a wildcard.**
  D-F2F.4 allow-lists exactly one occurrence with a
  justification comment quoting R12. Any future legitimate hit
  adds one explicit line, not a regex catch-all.
- **`todo!()` or `unimplemented!()` in the bodies.** Workspace
  CLAUDE.md no-half-finished-implementation rule applies. If a
  stage cannot complete, mark it `[!]` and halt — do not commit
  a placeholder body that compiles but does nothing.
- **`--no-verify` to skip a failing hook.** Never. Fix the
  cause.
- **Touching `starter-flow-spi`.** Phase 1 froze it. Stage 7
  baseline diff catches drift.
- **Touching `starter-flow-surfaces`.** That's Phase 3's job;
  not here.

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
   the job's branch (`codeless/starter-flow-engine-finish`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
