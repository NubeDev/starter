## Done

- stage 14 (phase D.3) — Goal 3 skill + flow YAML + integration test + design doc, committed in two commits on codeless/rubix-goals-2-4-3
- commit 1 (`8865c56`): refreshed `rubix-skills/skills/flow-programmer/SKILL.md` to present-tense covering the four `rubix.flow_ops.*` verbs plus `rubix.undo.last`; populated `rubix-flows/flows/flow-programmer.yaml` `allowed_tools[]` with the same five tools
- commit 2 (`83e8992`): added `rubix-agent/tests/goal_3_flow_programmer_test.rs` (seeds scheduled-system-check, duplicates via UndoDispatcher, asserts new revision row + `flow_ops.list` surfaces both, fires `rubix.undo.last`, asserts list reverts to source alone) and `docs/design/flow-programmer/README.md` covering verb surface, deploy contract, the `pg_notify('rubix_flows_definitions')` NOTIFY mechanism, and `FlowDefChange` snapshot shape
- `cargo test -p rubix-agent --test goal_3_flow_programmer_test` green; `cargo test -p rubix-flows` green; `bash rubix/scripts/lint-doc-refs.sh` clean

## Next

- (none) — fresh session picks up stage 15

## What you need to know

- Tool names are `rubix.flow_ops.{deploy,lint,list,duplicate}` (not the short `flow.*` shorthand the stage brief uses); both the SKILL.md `allowed_tools` and the bundled flow YAML use the full names so AgentLoop's ToolSet filter matches
- Test uses the same `UndoDispatcher` seam goal_2 and goal_4 tests use (the "via MCP" wording is shorthand — actual MCP-stdio coverage waits on `boot::mcp::register::build_flow_registry` wiring per the test doc-comment and the user-admin design doc)
- Duplicate's `FlowDefChange::prior_revision_id` is always `None`, so undo leaves the target `flow_id` with no live revision (asserted in the test)

## Open questions

- (none)
