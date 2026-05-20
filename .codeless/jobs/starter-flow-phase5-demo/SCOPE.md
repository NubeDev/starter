# Scope — starter-flow-phase5-demo

> Source of truth:
> [`DOCS/flow/scope/SCOPE.md`](../../../DOCS/flow/scope/SCOPE.md)
> §"Phase 5 — Remaining built-in node kinds" plus the R1 (everything
> is a node), R2 (one write chokepoint), R10 (reverse-DNS ids), R12
> (observability), and R13 (cancellation) rule blocks. This file is
> the per-job brief; intentionally short. When this file disagrees
> with the source-of-truth SCOPE, that doc wins.

## Goal

Land an end-to-end demo of the Phase 4 `ai-agent` node on a real
host (`examples/notes`) by shipping the two cheapest companion node
kinds — `trigger.explicit` and `log` — and wiring a one-button
demo flow against the Claude runner that already lives in
`starter-ai`. The demo proves the Codeless shape ("trigger →
ai-agent → sink") on one engine, end-to-end, in the same host
where Phase 1 i18n + Phase 3 prefs already ship. The remaining
eight Phase 5 node kinds (`branch`, `merge`, `subflow`, `gate`,
`trigger.{event, schedule, webhook}`, `http-out`, `sleep`) stay
stubbed; their D-F5 decisions land in a follow-up job when a real
consumer surfaces a need. Phase 5's full "Codeless and Rubix shape
on one engine" smoke is **not** in scope here — the Rubix shape
needs `branch` + `merge` + `http-out` which this job does not
ship. This job ships the Codeless half of that smoke as the
end-to-end demo.

## Out of scope

- The other eight Phase 5 node kinds. Their stubs stay at 14
  lines each; their feature flags stay `= []`. A follow-up job
  picks them up when a consumer needs them.
- Visual canvas (`starter-ui-flow`) — Phase 8.
- Engine refactors. The Phase 4 `AiAgent::with_provider_id`
  workaround stays in place; whatever propagator change retires
  it is Phase 6 territory.
- `starter-skills` real selector. Phase 4 shipped
  `NullSkillSelector`; this demo runs with that default.
- Real Claude API calls in CI. The end-to-end smoke uses the
  `RecordingAiRunner` testkit from `starter-ai`'s `testing`
  feature. The notes host runs the real Claude runner when a
  user fires the button locally; CI never hits the network.

## Deliverables

1. **`trigger.explicit` node body** in
   `crates/starter-flow-nodes/src/trigger_explicit.rs` behind a
   new default-off `trigger-explicit` cargo feature on
   `starter-flow-nodes`. The body exposes a `fire(payload)`
   handle the host calls; no axum/cron substrate.
2. **`log` node body** in `crates/starter-flow-nodes/src/log.rs`
   behind a new default-off `log` cargo feature on
   `starter-flow-nodes`. The body writes its input slot as a
   structured `tracing` event through the existing observability
   seam (R13).
3. **Demo wiring in `examples/notes/`** — register the Claude
   runner from `starter-ai` against the `ai-agent` node's
   `AiRunnerRegistry`, register both new node kinds, ship a flow
   definition (`trigger.explicit → ai-agent → log`), add a UI
   button (or CLI subcommand — TBD at stage 4) that fires the
   trigger, surface the log output in the notes UI.
4. **Dep-tree gates** — two new integration tests in
   `crates/starter-flow/tests/workspace_dep_tree_gates.rs`:
   `starter_flow_nodes_with_trigger_explicit_feature_does_not_pull_adk_rust`
   and `starter_flow_nodes_with_log_feature_does_not_pull_adk_rust`.
   Mirrors the stage-6 Phase-4 gate shape verbatim. The seven
   prior gates (six dep-tree + one SPI baseline) stay green.
5. **End-to-end smoke** in
   `crates/smoke-tests/tests/codeless_shape_on_one_engine.rs`:
   builds a three-node flow (`trigger.explicit` →
   `ai-agent` (RecordingAiRunner) → `log`), fires the trigger,
   asserts the ai-agent's recorded call saw the trigger payload,
   asserts the log node emitted exactly one structured event
   with the ai-agent's output. Single commit per D-F4.10
   precedent.

## Non-negotiable invariants

- **R1 — everything is a node.** Both new bodies impl
  `NodeBehavior`. The host-side `fire` handle for
  `trigger.explicit` is a thin wrapper around the standard
  invoke entry; it does not bypass the propagator.
- **R2 — one write chokepoint.** Both bodies return their
  output `SlotMap`; the propagator funnels through
  `GraphStore::write_slot`. Neither body writes a slot directly.
- **R5 — stateless behaviours.** Both bodies build
  per-invocation context fresh inside `invoke`; any host-shared
  state (the explicit-fire channel) hangs off an `Arc` field on
  the body struct.
- **R10 — reverse-DNS ids.** Kind ids
  `starter.flow.trigger.explicit` and `starter.flow.log`
  are already locked in the stubs and do not change.
- **R12 — observability.** `trigger.explicit` opens a
  `trigger_explicit.invoke` span; `log` opens a `log.invoke`
  span. Both record `(node_id, run_id, principal_id_hash,
  cancel_observed)` at minimum.
- **R13 — cancellation.** Both bodies select against
  `ctx.cancel.cancelled().await`. `trigger.explicit` cancels
  while waiting on its fire-channel; `log` cancels mid-emit
  (sub-millisecond budget — log is sync work).
- **D1 — adk-rust stays out.** The two new dep-tree gates
  enforce this on the `trigger-explicit` and `log` feature
  paths.
- **All 16 prior smokes stay green.** Phase 2 (3) + Phase 3 (4
  files, 13 tests) + Phase 4 (2 files, 3 tests). This job adds
  one new smoke at stage 5, bringing the total to 17 tests
  across 8 files (3 Phase 2 + 4 Phase 3 + 2 Phase 4 + 1 this
  job).
