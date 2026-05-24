## Done

- Implemented Goal 3 verbs in rubix-tools/src/flow_ops/: deploy.rs (validates via rubix_flows::parse_yaml, writes new revision and supersedes prior — Reversible), lint.rs (read-only; structured LintDiagnostic with serde_yaml line/column), list.rs (read-only; live flows sorted by flow_id), duplicate.rs (clones latest live revision under new id, rewrites body `id:` field — Reversible)
- Added rubix-tools/src/flow_ops/store.rs: FlowDefStore trait + InMemoryFlowDefStore + FlowDefReversible (kind = "flow_definition"); inverse marks new revision superseded + clears prior's superseded_at
- Filled the four flow_ops DTO modules in rubix-spi with request/response types + descriptors
- Added 6 MessageKeys to en.json + es.json: rubix.flow.deployed, rubix.flow.deploy.invalid, rubix.flow.linted, rubix.flow.lint.found_errors, rubix.flow.listed, rubix.flow.duplicated
- Added rubix-flows dep to rubix-tools/Cargo.toml; chrono dev-dep for Reversible test
- 16 new unit tests green (cargo test -p rubix-tools --lib flow_ops); full rubix-tools suite 63/63 green
- Committed as aca486d "stage 13: phase D.2 — Goal 3 verbs — feat(rubix-tools) flow-programmer verbs"

## Next

- Phase D.3 — wire the flow-programmer skill + flow YAML (allowed_tools = [rubix.flow_ops.deploy, lint, list, duplicate, rubix.undo.last]), and add the goal-3 integration test in rubix-agent/tests/goal_3_flow_programmer_test.rs that drives deploy → undo through the MCP loop, plus the design doc under docs/design/flows/
- Eventually swap InMemoryFlowDefStore for a PG-backed FlowDefStore impl that targets the flows_definitions table (the migration + NOTIFY trigger landed in stages 11–12)

## What you need to know

- FlowDefReversible only handles Op::Create (deploy + duplicate both Create); other ops error
- Snapshot shape lives in `Change::after` only (no `before`); it's a `FlowDefChange { flow_id, revision_id, prior_revision_id }`
- duplicate sets `prior_revision_id: None` in its change_for because it writes the first revision under a new flow_id (the source's row is untouched)
- duplicate rewrites the body via a line-based `id:` swap, then re-parses through parse_yaml to confirm — keeps comments/ordering stable
- lint serializes serde_yaml `location()` when available; other LoadError variants render with no line
- The shared in-memory store keeps superseded rows in place so undo can clear `superseded_at` to restore them

## Open questions

- (none)
