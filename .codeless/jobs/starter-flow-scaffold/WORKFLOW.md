# Workflow — starter-flow-scaffold

How to drive this job. The shape is "land the contracts crate in
full, then three empty-skeleton crates that compile, then wire
them into the workspace and prove the dep-tree gates pass."

## Sequencing

- Stage 1 is **prose-only**. Lock the four scaffold-relevant
  questions in [SCOPE.md](./SCOPE.md), record under "Decisions",
  commit. No code.
- Stage 3 lands `starter-flow-spi` in full — the Phase 1
  contracts crate from the flow SCOPE. This is the only stage
  with substantive code in this job; every later stage depends
  on its types.
- Stages 4 / 5 / 6 land the three skeleton crates. They can in
  principle land in any order after stage 3, but the WORKFLOW
  pins the order `starter-flow` → `starter-flow-nodes` →
  `starter-flow-surfaces` because each is conceptually narrower
  than the last (engine module map → kind feature map → two
  surface structs).
- Stage 7 wires the workspace and runs the dep-tree gates. A
  baseline mismatch here is the cheapest signal that a stage
  pulled in something it should not have.

## Per-stage discipline

- Before any code change in a stage:
  - `git log -20 --oneline` for the surrounding history.
  - Re-read the rule numbers in [SCOPE.md](./SCOPE.md) the
    stage touches. R2 (`WriteSlotOpts.replay`), R5
    (`&self` discipline), R7 (no workflow-agent types
    anywhere), R8 (two traits, not one), R10 (reverse-DNS) are
    the load-bearing rules even at scaffold time.
  - Re-read the SCOPE section that names the module you are
    creating. Doc comments at the head of every new module
    reference that section by name so a future reader does not
    re-derive the design from code.
- Touch only what the stage names. No drive-by refactors.
- Verify before commit:
  - **Rust**: `cargo check -p <touched crate>`, then
    `cargo test -p <touched crate>` (most stages have only
    doctest harness on the contracts crate), then
    `cargo clippy --workspace --all-targets -- -D warnings`,
    then `cargo fmt --check`.
  - **Dep-tree**: every stage that touches `starter-flow-spi`
    re-runs `cargo tree -p starter-flow-spi --edges normal` and
    diffs against the committed baseline. A diff means either
    the baseline updates (a separate, reviewed commit) or the
    change is rolled back.
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

One:

- **After stage 1** — decisions sign-off before any crate
  scaffolds land. Four small questions (default-features
  posture, node-kind feature list, baseline location,
  ext-flow placement) shape every later stage; locking them
  down first is cheap. Stage 3 lands a full contracts crate
  whose surface is hard to change after consumers depend on
  it, so the REVIEW here protects the Phase-1 boundary.

Stage 7 is itself a verification stage — the cargo-tree
snapshot pass is the merge gate, not a third REVIEW.

Write a one-line summary into the handover at the gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for all four open questions; no code changed; the four feature names locked match the SCOPE's built-in node-kinds enumeration exactly. |
| 3 | `starter-flow-spi` compiles, depends only on `starter-spi`, exposes `node` / `graph` / `flow` / `skill` modules per the SCOPE's What-lands-in-starter-flow-spi block; `WriteSlotOpts.replay: bool` is present; `NodeBehavior::invoke` takes `&self`; `#[non_exhaustive]` is on every public enum and config struct; the `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` snapshot is committed and the diff is clean. |
| 4 | `starter-flow` compiles with `default-features = []`; `lib.rs` declares `graph`, `registry`, `propagator`, `engine`, `run`, `state` modules; each module file is empty except for a doc comment naming the SCOPE section that owns it and a `// Phase N` marker; `cargo check -p starter-flow` green; no `async fn` appears anywhere in the crate's `src/`. |
| 5 | `starter-flow-nodes` compiles under `--no-default-features`; one cargo feature per built-in kind matches the locked list exactly; the `all-kinds` aggregate feature enables every kind; each kind module exposes only its `KIND_ID: &str` constant in the reserved `starter.flow.*` namespace; `cargo check -p starter-flow-nodes --features all-kinds` green; no `NodeBehavior` impl block exists in the crate. |
| 6 | `starter-flow-surfaces` compiles; `FlowAsTool` and `FlowAsService` are declared as empty structs with doc comments naming R8 and R9 respectively; `grep -rn 'todo!()' crates/starter-flow-surfaces/src` yields nothing; no `impl Tool for FlowAsTool` and no `impl Service for FlowAsService` block exists. |
| 7 | `starter/Cargo.toml` workspace `members` list includes the four new crates; `cargo build --workspace --all-features` green; `cargo clippy --workspace --all-targets -- -D warnings` green; `cargo fmt --check` green; `cargo tree -p starter-flow-spi --edges normal` matches `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`; `cargo tree -p starter-flow --edges normal` and `cargo tree -p starter-flow-nodes --edges normal` contain no `adk-rust` entry; the four new crates do not depend on `starter-mcp` / `starter-server` / `starter-cli`. |

## Anti-patterns

- Adding engine logic to `starter-flow` during the scaffold.
  The crate ships with empty module placeholders; the engine
  lands in Phase 2 of the flow SCOPE, in its own job. A
  scaffold that bakes in design assumptions is harder to
  change at Phase 2 than an empty placeholder is to fill.
- Adding `todo!()` or `unimplemented!()` bodies to
  `FlowAsTool` / `FlowAsService` trait impls. CLAUDE.md's
  no-half-finished-implementation rule applies; declare the
  struct, omit the trait impl. A consumer who tries to use
  `FlowAsTool` today gets a compile error pointing at a
  missing trait impl, which is the correct signal.
- Importing `adk-rust` anywhere. D1 is unresolved; the
  scaffold must not preemptively decide either side. The
  `workspace-builds-without-adk-rust` cargo-tree snapshot is
  the CI gate.
- A second tool registry, a second event bus, a second IPC.
  Per the flow SCOPE What-does-NOT-land block; the contracts
  crate does not declare these because the engine reuses the
  existing seams.
- Pre-implementing one or more built-in node kinds "while we
  are here". The kind modules expose only their id constants
  in this scaffold. The `transform` and `tool-call` bodies
  land in Phase 2 of the flow SCOPE; the `ai-agent` body in
  Phase 4; the rest in Phase 5. Conflating them here would
  collapse the phase boundary R7 protects.
- Skipping the `WriteSlotOpts.replay: bool` flag in
  `starter-flow-spi` because "we'll add it when the engine
  needs it." R2's replay-does-not-re-fire-subscribers
  semantics are part of the contract; ship the flag now so
  Phase 2 cannot retro-fit a different shape.
- Putting policy slot names (`session_policy`, `on_failure`,
  `cost_cap`, `safe_state`, `trigger`, `auth`, `timeout`) in
  the contracts crate as enums. R3 — the engine is a reader
  of policies, not an owner. Policy values are ordinary slot
  values; the engine reads them at the moment they apply.
  Enumerating them in `starter-flow-spi` would tilt the
  design toward engine-owned policy.
- Pulling the `starter-ext-flow` adapter into this job
  because "the flow SCOPE mentions it." It lives in
  `starter-extensions/` per the SCOPE's
  Relationship-to-existing-crates block; landing it here
  would couple two workspaces.
- Re-exporting `starter-skills::SkillSelection`
  unconditionally. The `skills` feature is default-off
  because `starter-skills` is a planned crate the workspace
  does not yet ship.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
   On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to the
   job's branch (`codeless/starter-flow-scaffold`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
