# Handover — starter-flow-phase5-demo

## Current stage

**Bootstrap on `master`; ready for stage 1 (D-F5 decisions) on the
branch `codeless/starter-flow-phase5-demo`.** The job folder is
in place; no SCOPE.md edits yet — those land at stage 1.

## Why this job exists

User pivot during planning: instead of shipping all ten Phase 5
node kinds in one big push, ship just the two cheapest companions
to the Phase 4 `ai-agent` node (`trigger.explicit` + `log`) and
wire them into the existing `examples/notes` host as an
end-to-end demo against the Claude runner from `starter-ai`. The
remaining eight Phase 5 node kinds (`branch`, `merge`, `subflow`,
`gate`, `trigger.{event, schedule, webhook}`, `http-out`,
`sleep`) stay stubbed; a follow-up job picks them up when a
consumer surfaces a real need.

## Phase 4 inheritance — what stays green

This job inherits a 24/7-rated persistent runtime running real
agentic flows end-to-end. Every gate Phase 4's stage-9 verify
pass confirmed must stay green:

- Six workspace dep-tree gates
  (`starter_flow_spi_baseline_holds`,
  `starter_flow_tree_contains_no_adk_rust`,
  `starter_flow_nodes_tree_contains_no_adk_rust`,
  `starter_flow_surfaces_tree_contains_no_adk_rust`,
  `no_flow_crate_depends_on_phase3_surfaces`,
  `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`).
- Three Phase 2 smokes under `crates/starter-flow/tests/`.
- Four Phase 3 smokes (13 `#[tokio::test]`s) under
  `crates/smoke-tests/tests/`.
- Two Phase 4 smokes (3 tests) under
  `crates/smoke-tests/tests/`.
- `cargo test` on `starter-flow`, `starter-flow-spi`,
  `starter-flow-surfaces`, `starter-flow-nodes --features
  ai-agent,tool-call`, `starter-store-sqlite --features
  flow,testing` all green.
- Flow-scoped `cargo clippy ... --all-targets -- -D warnings`.
- Flow-scoped `cargo fmt --check`.

This job adds two new dep-tree gates (stages 2 + 3) and one new
SCOPE smoke (stage 5); stage 6 verify pass re-runs the full set.

## Stages

- **Stage 1.** Lock D-F5.1..D-F5.5 in
  `DOCS/flow/scope/SCOPE.md`. Commit on branch; no code.
- **Stage 2.** `trigger.explicit` body in
  `crates/starter-flow-nodes/src/trigger_explicit.rs` behind
  default-off `trigger-explicit` cargo feature + new dep-tree
  gate.
- **Stage 3.** `log` body in
  `crates/starter-flow-nodes/src/log.rs` behind default-off
  `log` cargo feature + new dep-tree gate.
- **Stage 4.** Wire demo into `examples/notes/` — register
  Claude runner + node kinds, define flow YAML, add fire
  endpoint + UI button.
- **Stage 5.** End-to-end smoke
  `codeless_shape_on_one_engine.rs` driven by
  `RecordingAiRunner` (no CI network).
- **Stage 6.** Workspace verify + dep-tree gates re-confirm +
  PR.

## Branch + commits

- Branch: `codeless/starter-flow-phase5-demo` (created before
  stage 1 commit; the bootstrap commit lands on `master` per
  Phase 4 precedent — `.codeless/jobs/*` is doc-only).
- Bootstrap commit: forthcoming — `phase 5 demo: bootstrap job
  folder`.

## D1 reminder

D1 was resolved at stage 1 of the merged Phase 3 sibling job
and re-confirmed by D-F4.2: `starter-flow-node-loop` shape
wins; adk-rust stays out of the workspace dep tree. Every stage
of this job must keep the six existing `*_contains_no_adk_rust`
+ baseline gates green, and stages 2 + 3 add two more gates
covering the `--features trigger-explicit` and `--features
log` opt-in paths.
