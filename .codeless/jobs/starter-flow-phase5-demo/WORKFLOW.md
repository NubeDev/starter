# Workflow — starter-flow-phase5-demo

How to drive the Phase 5 demo with the same rigor Phase 3 + Phase 4
established: decisions locked at the entry gate, bodies behind
default-off cargo features, dep-tree gates re-confirmed at the end,
one commit per stage. This job is **smaller** than Phase 4 — two
node bodies (~50 lines each, no LLM loop, no skill plumbing) plus a
demo wiring stage and a single end-to-end smoke. Six stages total
versus Phase 4's nine.

## Sequencing

- **Stage 1 is prose-only.** Lock D-F5.1 through D-F5.5 in
  [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
  under "Decisions" (precedent: Phase 4 stage 1). Commit on
  branch; no code. The bootstrap commit (job folder only) lands
  on `master` per Phase 4 precedent — `.codeless/jobs/*` is
  doc-only.
- **Stage 2 lands the `trigger.explicit` body** in
  `crates/starter-flow-nodes/src/trigger_explicit.rs` behind a
  new default-off `trigger-explicit` cargo feature on
  `starter-flow-nodes`. Unit tests cover each invariant
  (fire-channel wakes the body, cancel before fire surfaces
  `NodeError::Cancelled`, R12 span shape). New dep-tree gate
  `starter_flow_nodes_with_trigger_explicit_feature_does_not_pull_adk_rust`
  lands in the same commit.
- **Stage 3 lands the `log` body** in
  `crates/starter-flow-nodes/src/log.rs` behind a new
  default-off `log` cargo feature on `starter-flow-nodes`. Unit
  tests cover each invariant (input slot becomes a structured
  `tracing` event, R12 span shape, sync work cancels cleanly).
  New dep-tree gate
  `starter_flow_nodes_with_log_feature_does_not_pull_adk_rust`
  lands in the same commit.
- **Stage 4 wires the demo into `examples/notes/`.** Register
  the Claude runner from `starter-ai` against an
  `AiRunnerRegistry`, register the two new node kinds + the
  `ai-agent` + `tool-call` kinds, define the demo flow
  (`trigger.explicit → ai-agent → log`), wire a UI button (or
  CLI subcommand — decide at stage entry based on what the
  notes example already exposes) that calls the explicit-fire
  handle. The flow definition lives next to the existing
  notes flow assets if any, or under
  `examples/notes/flows/codeless-demo.yaml` (TBD per the notes
  layout at stage entry).
- **Stage 5 ships the end-to-end smoke** in
  `crates/smoke-tests/tests/codeless_shape_on_one_engine.rs`:
  the same three-node flow shape stage 4 wires into notes, but
  driven by `RecordingAiRunner` so CI never hits the network.
  Asserts trigger-payload→ai-agent input plumbing and
  ai-agent-output→log event plumbing. Single commit per
  D-F4.10 precedent.
- **Stage 6 is workspace verify** + dep-tree gates re-confirm
  (Phase 3 stage 10 / Phase 4 stage 9 shape). No code; just
  running the gates and documenting pass/fail per gate in the
  handover. Then PR against master.

## What stays green from Phase 4

- Six existing workspace dep-tree gates
  (`starter_flow_spi_baseline_holds`,
  `starter_flow_tree_contains_no_adk_rust`,
  `starter_flow_nodes_tree_contains_no_adk_rust`,
  `starter_flow_surfaces_tree_contains_no_adk_rust`,
  `no_flow_crate_depends_on_phase3_surfaces`,
  `starter_flow_nodes_with_ai_agent_feature_does_not_pull_adk_rust`).
- Three Phase 2 smokes under `crates/starter-flow/tests/`
  (`smoke_one_write_chokepoint`,
  `smoke_engine_is_reader_of_policies`,
  `r3_no_policy_match_arms`).
- Four Phase 3 smokes under `crates/smoke-tests/tests/`
  (`flow_via_mcp`, `flow_as_service`,
  `flow_event_stream_over_four_transports`,
  `flow_crash_and_resume`) — 13 tests.
- Two Phase 4 smokes under `crates/smoke-tests/tests/`
  (`ai_agent_is_just_a_node_kind`, `skill_quarantine_*`) — 3
  tests.
- `cargo test` on `starter-flow`, `starter-flow-spi`,
  `starter-flow-surfaces`, `starter-flow-nodes --features
  ai-agent,tool-call`, `starter-store-sqlite --features
  flow,testing` all green.
- `cargo clippy` on the flow-scoped crates with `-D warnings`.
- `cargo fmt --check` on the flow-scoped globs.

## Commit shape

One commit per stage; bundling is a stage-fail. Commit message
format mirrors Phase 4:

```
stage N: <one-line summary>

<two-to-four-paragraph body explaining what landed, what was
chosen and why, and what was deliberately deferred>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Branch: `codeless/starter-flow-phase5-demo`. Push after every
stage commit so a fresh session can resume from the handover +
the remote branch state.
