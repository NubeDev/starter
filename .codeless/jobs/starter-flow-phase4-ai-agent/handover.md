# Handover — starter-flow-phase4-ai-agent

## Current stage

**Stage 1 — lock the Phase 4 boundary (no code).** Job folder
bootstrapped on `master` at `af52c7b` (immediately after the
Phase 3 stage 10 verify pass merged). Stage 1's work is to record
the twelve D-F4 decisions in `DOCS/flow/scope/SCOPE.md` under
"Decisions" per the Phase 3 stage 1 precedent. No code lands until
the stage 2 REVIEW gate signs off.

## Phase 3 inheritance — what stays green

This job inherits a 24/7-rated persistent runtime. Every gate the
Phase 3 stage 10 verify pass confirmed must stay green through
every Phase 4 stage:

- Five workspace dep-tree gates
  (`starter_flow_spi_baseline_holds`,
  `starter_flow_tree_contains_no_adk_rust`,
  `starter_flow_nodes_tree_contains_no_adk_rust`,
  `starter_flow_surfaces_tree_contains_no_adk_rust`,
  `no_flow_crate_depends_on_phase3_surfaces`).
- Three Phase 2 smokes under `crates/starter-flow/tests/`.
- Four Phase 3 smokes (13 `#[tokio::test]`s) under
  `crates/smoke-tests/tests/`.
- `cargo test` on `starter-flow`, `starter-flow-spi`,
  `starter-flow-surfaces`, `starter-store-sqlite --features
  flow,testing` all green.
- Flow-scoped `cargo clippy ... --all-targets -- -D warnings`.
- Flow-scoped `cargo fmt --check`.

Phase 4 adds one new dep-tree gate (stage 6) and two new SCOPE
smokes (stages 7 + 8); the final stage 9 verify pass re-runs the
full set.

## Stages

- **Stage 1.** Lock D-F4.1..D-F4.12 in
  `DOCS/flow/scope/SCOPE.md`. Commit; no code.
- **Stage 2.** REVIEW gate (user sign-off).
- **Stage 3.** SPI shape — `AiRunnerRegistry`, `SkillSelector`,
  `SkillSelection`, `SessionMode`, additive `NodeCtx { ...,
  skill }` field. Baseline regen if needed (D-F3.7).
- **Stage 4.** `ai-agent` body in
  `crates/starter-flow-nodes/src/ai_agent.rs` behind default-off
  `ai-agent` cargo feature. Unit tests per invariant.
- **Stage 5.** Engine wiring —
  `Engine::with_ai_runner_registry`,
  `Engine::with_skill_selector`,
  `FlowRunner::start` runs the selector once and threads through
  every `NodeCtx`.
- **Stage 6.** Sixth dep-tree gate:
  `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`.
- **Stage 7.** Smoke 1 — `ai_agent_is_just_a_node_kind.rs` +
  `RecordingAiRunner` testkit in `starter-ai` `testing` feature.
- **Stage 8.** Smoke 2 —
  `skill_quarantine_survives_bundle_update_through_a_flow.rs`.
- **Stage 9.** Workspace verify + dep-tree gates re-confirm
  (Phase 3 stage 10 shape).

## Branch + commits

- Branch: `codeless/starter-flow-phase4-ai-agent` (to be created
  before stage 1 commit; the bootstrap commit lands on `master`
  per workflow precedent — `.codeless/jobs/*` is doc-only).
- Bootstrap commit: forthcoming — `phase 4 ai-agent: bootstrap
  job folder`.

## D1 reminder

D1 was resolved at stage 1 of the merged Phase 3 sibling job:
`starter-flow-node-loop` shape wins; adk-rust stays out of the
workspace dep tree (see `DOCS/flow/scope/SCOPE.md:754`). D-F4.2
re-confirms this with the revisit trigger named verbatim. Every
stage of this job must keep the existing four
`*_contains_no_adk_rust` gates green, and stage 6 adds a sixth
gate that confirms the invariant holds on the opt-in
`--features ai-agent` path too.
