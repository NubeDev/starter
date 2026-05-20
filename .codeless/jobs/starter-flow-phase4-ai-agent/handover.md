# Handover — starter-flow-phase4-ai-agent

## Current stage

**Phase 4 complete on branch `codeless/starter-flow-phase4-ai-agent`;
ready to open PR against master.** All nine stages landed on the
branch with the gate-set re-confirmed green at stage 9. The
`ai-agent` node kind body, the engine wiring, the new dep-tree
gate, and the two SCOPE smokes are in place; the engine can run
agentic flows end-to-end on the 24/7-rated persistent runtime
Phase 3 shipped.

## Stage 9 outcome (verify pass)

| Gate | Result |
|---|---|
| workspace dep-tree gates (now 6 including ai-agent feature path) | **pass** — 6/6 |
| starter-flow tests | **pass** — every binary green (Phase 2 smokes + stage5/6/7 tests + stage5_skill_threading 4/4) |
| starter-flow-spi tests | **pass** — 11/11 |
| starter-flow-surfaces tests | **pass** — stage7 6/6, stage8 5/5 |
| starter-flow-nodes --features ai-agent,tool-call | **pass** — 13/13 (6 ai-agent + 7 tool-call) |
| starter-store-sqlite --features flow,testing | **pass** — flow 8/8, migrate 3/3 |
| Phase 3 smokes (4 files, 13 tests) | **pass** — all green |
| Phase 4 smokes (2 files, 3 tests) | **pass** — ai_agent_is_just_a_node_kind 1/1, skill_quarantine 2/2 |
| flow-scoped clippy --features ai-agent,tool-call | **pass** — 0 warnings |
| flow-scoped fmt --check | **pass** — exit 0 |

## Commits (in order)

- `d1a4909` master — bootstrap job folder.
- `deb9e94` branch — stage 1: D-F4.1..D-F4.12 in SCOPE.md.
- `0db2123` branch — stage 3: SPI shape (AiRunnerRegistry, SkillSelector, SkillSelection, SessionMode, NodeCtx.skill, baseline regen).
- `42499d5` branch — stage 4: ai-agent body behind default-off `ai-agent` feature + 6 unit tests.
- `5cdaee4` branch — stage 5: Engine builder hooks + selector threading through NodeCtx + stage5_skill_threading 4 tests.
- `a7b3788` branch — stage 6: sixth dep-tree gate for the --features ai-agent path.
- `0c1c6da` branch — stage 7: smoke 1 + RecordingAiRunner testkit (starter-ai testing feature).
- `828729e` branch — stage 8: smoke 2 (skill quarantine + cross-run mutation).
- (this commit) branch — stage 9: verify pass + handover update.

## Implementation notes worth remembering

- **`AiAgent.with_provider_id` is a Phase-4 workaround.** The
  Phase 2 propagator only routes declared trigger slots into a
  node body's `input` map; a `provider_id` declared as a non-
  trigger config slot is invisible. The smoke + downstream
  consumers pin the provider at construction time. Retire this
  when NodeCtx gains a graph-store reference so the body can read
  config slots directly (likely Phase 5/6).
- **`SessionId::for_ai_agent_node` uses uuid v5** over a frozen
  namespace (constant `SESSION_NS` in starter-flow-spi flow.rs).
  Workspace `uuid` feature gained `v5` in stage 3, pulling
  `sha1_smol` into the SPI tree — baseline regenerated in same
  commit per D-F3.7.
- **CancelAdapter doesn't impl AiCancel.** The trait requires
  `'static`; a borrowed adapter can't satisfy it. The outer
  `tokio::select!` in `run_agent_loop` races the runner future
  against `ctx.cancel.cancelled()` directly; the AiRunner
  receives a static `NoOpAiCancel`. Cancel-to-exit ≤ 200ms
  (asserted in stage 4 unit tests).
- **`SkillError` from the selector is non-fatal** — the runner
  logs a warn and falls back to `SkillSelection::None`. Matches
  the Phase 2 "selector failure is not fatal" posture. Revisit
  when a host surfaces a need to hard-fail (D-F4.4 revisit
  trigger).
- **The R12 span shape is unit-tested in the body**, not in the
  smoke. Cargo's process-wide tracing-subscriber default fights
  with cross-thread propagator dispatch; the smoke pins the
  end-to-end FlowTopology → propagator → NodeBehavior::invoke →
  GraphStore::write_slot chain.
- **AiAgent body is ~900 lines** with 6 inline unit tests; the
  bulk is config validation + tools intersection + the LLM loop
  + the cancel adapter + session resolution. tool_call.rs is
  the structural precedent (633 lines).

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
