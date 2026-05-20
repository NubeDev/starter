# Scope — starter-flow-phase4-ai-agent

> Source of truth:
> [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> §"Phase 4 — `ai-agent` node kind (D1 resolution)" plus the R7 (the
> AI agent is a node kind, not a runtime), R8 (Nodes are not Tools),
> and the "Skills bind to the `ai-agent` node kind" rule blocks. This
> file is the per-job brief; intentionally short. When this file
> disagrees with the source-of-truth SCOPE, that doc wins.

## Goal

Land Phase 4 of the flow SCOPE — the `ai-agent` node kind body —
with the D1 resolution already locked at stage 1 of the merged Phase
3 sibling job: **`starter-flow-node-loop` shape wins; adk-rust stays
out of the workspace dep tree.** Every LLM call from any `ai-agent`
node routes through `starter_spi::ai::AiRunner` (the existing
`starter-ai` impls satisfy this seam). Skill selection runs once per
outer flow run and threads through every `ai-agent` node as a frozen
`SkillSelection`; the per-node `skill_hint` config slot is the only
override. The tools allowlist for any ai-agent invocation is the
intersection of (host `ToolRegistry` ∩ skill `allowed_tools` ∩ node
`config.allowed_tools`). The Phase 4 SCOPE smokes ("AI agent is just
a node kind" + "skill quarantine survives bundle update through a
flow") pass; the existing five workspace dep-tree gates stay green,
and a sixth gate is added to confirm enabling the new `ai-agent`
cargo feature on `starter-flow-nodes` does not pull `adk-rust` into
the dep tree on the opt-in path either. No engine refactors beyond
the two additive builder hooks (`Engine::with_ai_runner_registry`,
`Engine::with_skill_selector`) and the additive `NodeCtx` field that
carries the frozen `SkillSelection` through every node invocation in
a run.

## Out of scope

- adk-rust adoption in any form — D-F4.2 locks the loop shape and
  the revisit trigger names the conditions under which a future
  Phase ships `starter-flow-node-adk` as a second opt-in body.
- The real `starter-skills` crate that ships a content-hash-backed
  `SkillSelector`. Phase 4 ships the `SkillSelector` trait + a
  default `NullSkillSelector` returning `SkillSelection::None`; the
  full skills bundle pipeline is its own follow-up job under the
  agent SCOPE.
- New surfaces (Phase 3 shipped `FlowAsTool` + `FlowAsService`,
  enough for MCP / REST / CLI clients to drive an ai-agent flow).
- Remaining built-in node kinds — Phase 5 territory (`branch`,
  `merge`, `subflow`, `gate`, the four triggers, `http-out`, `log`,
  `sleep`).
- Visual canvas (`starter-ui-flow`) — Phase 8.

## Deliverables

1. **SPI shape** — `AiRunnerRegistry` trait, `SkillSelector` trait,
   `SkillSelection` enum, `SessionMode` config, plus the additive
   `NodeCtx { ..., skill: &'a SkillSelection }` field. All
   `#[non_exhaustive]` per the established Phase 3 posture.
2. **`ai-agent` node body** in
   `crates/starter-flow-nodes/src/ai_agent.rs` behind a new
   default-off `ai-agent` cargo feature on `starter-flow-nodes`.
   The body lifts Codeless's `Runner` shape: a turn-based LLM loop
   that routes every model call through
   `starter_spi::ai::AiRunner` and dispatches model-emitted
   tool-calls back through the same `ToolRegistry` the `tool-call`
   body uses.
3. **Engine wiring** — `Engine::with_ai_runner_registry(...)` and
   `Engine::with_skill_selector(...)` builder hooks;
   `FlowRunner::start` runs `SkillSelector::select` exactly once
   per run and threads the result through every `NodeCtx` via the
   new additive field.
4. **Dep-tree gate** — sixth integration test in
   `crates/starter-flow/tests/workspace_dep_tree_gates.rs`:
   `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`.
5. **Two Phase 4 SCOPE smokes** under
   `crates/smoke-tests/tests/` (per D-F3.6 precedent):
   `ai_agent_is_just_a_node_kind.rs` and
   `skill_quarantine_survives_bundle_update_through_a_flow.rs`.
   Each uses a `RecordingAiRunner` testkit shipped from
   `starter-ai`'s existing `testing` feature.

## Non-negotiable invariants

- **R7 — node, not runtime.** The body owns the LLM loop and tool
  dispatch; topology (`subflow`, `branch`, `merge`) is the engine's
  job, not the body's.
- **R2 — one write chokepoint.** The body returns its output
  `SlotMap`; the propagator funnels it through
  `GraphStore::write_slot`. The body never writes a slot directly,
  including for streaming intermediate tokens (those ride
  `FlowEvent` on the run's broadcast, not a slot).
- **R5 — stateless behaviours.** The body holds `Arc<dyn ToolRegistry>`,
  `Arc<dyn AiRunnerRegistry>`, and the per-invocation context is
  built fresh per `invoke` call.
- **R10 — reverse-DNS ids.** Kind id is `starter.flow.ai-agent`
  (already locked in the stub). Provider ids are reverse-DNS
  `KindId`s validated at registration time.
- **R12 — observability.** Every invocation opens an
  `ai_agent.invoke` tracing span recording `(node_id, provider_id,
  principal_id_hash, run_id, skill_id_or_none,
  turn_count, tool_call_count, cancel_observed)`.
- **R13 — cancellation.** `NodeCtx::cancel` propagates into the
  AiRunner via a `starter_spi::ai::Cancel` adapter; cancel-to-exit
  bounded at ≤ 200 ms (same budget as `tool-call`).
- **D1 — adk-rust stays out.** The workspace dep-tree gates
  (five existing + one new) enforce this on both the
  default-feature and the `ai-agent`-feature-enabled paths.
- **D-F3.7 — SPI baseline.** If the new SPI types pull any new
  transitive deps onto `starter-flow-spi`, regenerate
  `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` in the same
  commit and name the regeneration in the commit message
  (precedent: stage 3 of Phase 3). Stages that produce baseline
  diffs without a SPI dep edit are stage-fails.
