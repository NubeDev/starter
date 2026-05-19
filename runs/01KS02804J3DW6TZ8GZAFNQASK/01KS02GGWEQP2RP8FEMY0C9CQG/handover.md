## Done

- Added four Phase 2 entry-gate decisions (D1, D1a, D1b, D1c) under "Decisions made" in DOCS/flow/scope/SCOPE.md, each citing the rule it derives from (R7, R1, GraphStore trait contract, R13).
- Removed the D1 bullet from "Open questions" since it is now decided.
- Committed as 5073c79 on branch codeless/starter-flow-engine. Docs-only change; no code touched.

## Next

- Stage 2 picks up Phase 2 implementation work (in-memory GraphStore impl, propagator, registries, engine state machine, FlowEvent stream, transform + tool-call node kinds) on the locked decisions.

## What you need to know

- D1 resolution is "starter-flow-node-loop" (option b). The `workspace builds without adk-rust` snapshot gate must stay green through Phase 2.
- In-memory store shape is explicitly Phase-2-only; Phase 3 SQLite impls are not pre-decided beyond the GraphStore trait.
- FlowEvent stream uses tokio::sync::broadcast semantics — lagged receivers observe `Lagged` rather than silently dropping. Adapters must handle that.
- RunOpts is named in the SCOPE as the override carrier for max_propagation_hops and idempotent_short_circuit at FlowRunner::start.

## Open questions

- (none)
