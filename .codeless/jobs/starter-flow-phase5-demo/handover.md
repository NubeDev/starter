# Handover — starter-flow-phase5-demo

## Current stage

**Phase 5 demo complete on branch
`codeless/starter-flow-phase5-demo`; ready to open PR against
master.** All six stages landed plus two mid-job extension
commits (stage 3b adding `RunnerInput::Cli` support to the
ai-agent body, stage 3c splitting `tool_registry` into an
always-compiled module so `ai-agent` and `tool-call` feature
paths are orthogonal). The codeless shape (`trigger.explicit →
ai-agent → log`) runs end-to-end through the engine in CI via
RecordingAiRunner and through `examples/notes` against the
local Claude Code CLI.

## Stage 6 outcome (verify pass)

| Gate | Result |
|---|---|
| workspace dep-tree gates (now 8 with `trigger-explicit` + `log` paths) | **pass** — 8/8 |
| starter-flow-nodes --features all-kinds (lib) | **pass** — 31/31 |
| Phase 2 + Phase 3 + Phase 4 + Phase 5 smokes | **pass** — 7 files, 12 tests across the flow suite |
| codeless_shape_on_one_engine smoke (Phase 5) | **pass** — 1/1 |
| starter-notes builds clean with the demo wired in | **pass** |
| flow-scoped clippy --features all-kinds | **pass** — 0 warnings |
| flow-scoped fmt --check | **pass** — exit 0 |

## Commits (in order)

- `45ec414` master — bootstrap job folder.
- `05cd198` branch — stage 1: D-F5.1..D-F5.5 in SCOPE.md.
- `3cebb6e` branch — stage 2: trigger.explicit body + 5th dep-tree gate.
- `bae901a` branch — stage 3: log body + 6th dep-tree gate.
- `9e50679` branch — stage 3b: ai-agent supports `RunnerInput::Cli`.
- `5fd33b7` branch — stage 3c: split ToolRegistry into always-compiled module.
- `7a76473` branch — stage 4: codeless-demo flow wired into examples/notes.
- `3a5b8bb` branch — stage 5: end-to-end codeless-shape smoke.
- (this commit) branch — stage 6: verify pass + handover update.

## Implementation notes worth remembering

- **`AiAgent::with_input_kind(AgentInputKind::Cli)`** is the
  required twin to `with_provider_id` whenever a CLI-shape
  runner (Claude Code) backs the agent — the propagator's
  same Phase 2 trigger-slot-only routing limitation means
  both must be pinned at construction time. D-F5.6 names the
  revisit trigger.
- **`tool_registry` is now always compiled** regardless of
  features. The trait + `StaticToolRegistry` live in
  `crates/starter-flow-nodes/src/tool_registry.rs`;
  `tool_call.rs` and `ai_agent.rs` both import from there.
  Downstream consumers can keep importing through
  `starter_flow_nodes::tool_call::{ToolRegistry,
  StaticToolRegistry}` — those are re-exports of the
  always-compiled path.
- **`TriggerChannelRegistry` is body-local**, not in the SPI.
  The body file holds the trait + `StaticTriggerChannelRegistry`
  + `TriggerSender`/`TriggerReceiver` types. No SPI baseline
  drift this job (D-F3.7 untouched).
- **The fire-before-start ordering** in
  `examples/notes/src/flow_demo.rs` and the stage-5 smoke is
  deliberate: the trigger body's mpsc receiver is bounded,
  so firing first guarantees the payload is queued before the
  body's recv awaits. No race window.
- **CI never hits Anthropic.** The stage-5 smoke uses
  `RecordingAiRunner`. The notes host's POST
  `/api/flows/codeless-demo/fire` endpoint hits the local
  Claude Code CLI when a developer fires it — `claude auth
  login` is the auth path.
- **The Rubix half of Phase 5's "Codeless and Rubix shape on
  one engine" smoke is deferred** to a follow-up job. It
  needs `branch` + `merge` + `http-out` which this job does
  not ship; the other eight Phase 5 node kinds stay stubbed
  at 14 lines each.

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
