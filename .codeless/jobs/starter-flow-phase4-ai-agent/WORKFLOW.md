# Workflow — starter-flow-phase4-ai-agent

How to drive Phase 4 with the same Niagara-style rigor Phase 3
established: decisions locked at the entry gate, additive SPI shape
landed in one commit with the baseline regenerated in the same
commit if needed, body behind a default-off cargo feature, two SCOPE
smokes one-commit-per-file under `crates/smoke-tests/tests/`, dep-
tree gates re-confirmed at the end. The big difference from Phase 3:
**no durability hardening to land** — Phase 3 shipped the per-tick
checkpoint, retry-with-backoff, Degraded mode, dedup, and the
crash-and-resume smoke; Phase 4 inherits them and must not regress
any of those gates.

## Sequencing

- **Stage 1 is prose-only.** Lock D-F4.1 through D-F4.12 in
  [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
  under "Decisions" (precedent: Phase 3 stage 1). Commit; no code.
  No SPI edits in this commit either — the decisions name the
  trait shapes that stage 3 will land.
- **Stage 2 is the entry-gate REVIEW.** Do not advance until the
  user signs off — particularly on D-F4.2 (D1 resolution: loop
  shape vs deferred-adk), D-F4.4 (skill seam shape vs deferred-
  bundle pipeline), D-F4.5 (tools-intersection rule), D-F4.6
  (session mode default), and D-F4.10 (smoke location). These five
  cascade into every stage that follows and rework is expensive.
- **Stage 3 lands the SPI shape in one commit.** Trait method
  shapes per D-F4.3 + D-F4.4 plus the additive `NodeCtx` field
  per D-F4.4, the `SessionMode` enum per D-F4.6. Baseline
  regenerated in the same commit if the new types pull any
  transitive deps (per D-F3.7).
- **Stage 4 lands the `ai-agent` body** in
  `crates/starter-flow-nodes/src/ai_agent.rs` behind the new
  default-off `ai-agent` cargo feature. Unit tests cover each
  invariant in isolation (LLM loop turn ordering, tools
  intersection, session mode, cancellation, R12 span shape).
- **Stage 5 wires the engine.** `Engine::with_ai_runner_registry`
  + `Engine::with_skill_selector` builder hooks;
  `FlowRunner::start` runs the selector exactly once and threads
  the result through `NodeCtx`. Unit tests assert the selection
  is frozen across the run and that every node invocation in the
  run sees the same `SkillSelection`.
- **Stage 6 lands the new dep-tree gate.** Sixth integration
  test in `crates/starter-flow/tests/workspace_dep_tree_gates.rs`
  asserting `cargo tree -p starter-flow-nodes --features ai-agent
  --edges normal` contains zero `adk-rust` matches. The existing
  five gates stay green.
- **Stages 7 + 8 land the two SCOPE smokes.** One commit per
  smoke file per the Phase 3 stage-9 precedent — bundling is a
  stage-fail. The smokes use the `RecordingAiRunner` testkit
  from `starter-ai`'s `testing` feature (lands inline in stage 7
  if not already present).
- **Stage 9 is workspace verify** + dep-tree gates re-confirm
  (Phase 3 stage 10 shape). No code; just running the gates and
  documenting pass/fail per gate in the handover.

## What stays green from Phase 3

- All five existing workspace dep-tree gates
  (`starter_flow_spi_baseline_holds`,
  `starter_flow_tree_contains_no_adk_rust`,
  `starter_flow_nodes_tree_contains_no_adk_rust`,
  `starter_flow_surfaces_tree_contains_no_adk_rust`,
  `no_flow_crate_depends_on_phase3_surfaces`).
- Three Phase 2 smokes under `crates/starter-flow/tests/`
  (`smoke_one_write_chokepoint`,
  `smoke_engine_is_reader_of_policies`,
  `r3_no_policy_match_arms`).
- Four Phase 3 smokes under `crates/smoke-tests/tests/`
  (`flow_via_mcp`, `flow_as_service`,
  `flow_event_stream_over_four_transports`,
  `flow_crash_and_resume`) — 13 tests total.
- `cargo test -p starter-flow -p starter-flow-spi
  -p starter-flow-surfaces -p starter-store-sqlite
  --features flow,testing` all green.
- `cargo clippy -p starter-flow -p starter-flow-spi
  -p starter-flow-surfaces -p starter-smoke-tests
  -p starter-flow-nodes --all-targets -- -D warnings` clean
  (Phase 4 extends the scope to include `starter-flow-nodes`,
  which now ships a non-trivial body).
- `cargo fmt --check` on the flow-scoped globs.

## Commit shape

One commit per stage; bundling is a stage-fail (precedent: Phase 3
stage 9). Commit message format:

```
stage N: <one-line summary>

<two-to-four-paragraph body explaining what landed, what was
chosen and why, and what was deliberately deferred>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Branch: `codeless/starter-flow-phase4-ai-agent`. Push after every
stage commit so a fresh session can resume from the handover + the
remote branch state.
