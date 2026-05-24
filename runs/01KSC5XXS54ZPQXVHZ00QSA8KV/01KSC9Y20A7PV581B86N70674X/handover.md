## Done

- Updated `rubix/crates/rubix-skills/skills/clickhouse-ruler/SKILL.md` to present-tense, fixed dot-separated tool ids, added the four-entry `allowed_tools` list (three CH write verbs + `rubix.undo.last`), and added the data-loss caveat for undoing fresh marts.
- Populated `rubix/crates/rubix-flows/flows/clickhouse-ruler.yaml` with the same four-entry `allowed_tools` on the ai-agent node config.
- Added `rubix/crates/rubix-agent/tests/goal_4_clickhouse_ruler_test.rs` — one scenario that wires `ClickhouseRetentionSetTool` + `ChRetentionReversible` + `UndoLastTool` through `UndoDispatcher`, asserts the ALTER ran (writer state flipped 90→30), the changelog snapshot row has matching `before`/`after` `ChRetentionSnapshot` payloads, and `rubix.undo.last` restores 90d. `cargo test -p rubix-agent --test goal_4_clickhouse_ruler_test` passes.
- Added `rubix/docs/design/clickhouse-rules/README.md` (present-tense): verb surface table, backing-store trait note, per-kind snapshot shapes, the `mart.create` undo data-loss caveat, the six MessageKeys, and the idempotence-vs-undo contract.
- Two commits on `codeless/rubix-goals-2-4-3`:
- `d453f38` stage 10: phase C.2 — Goal 4 skill + flow YAML (feat(rubix-skills+rubix-flows))
- `4af90cd` stage 10: phase C.2 — Goal 4 integration test + design doc (test(rubix-agent) + docs(design))

## Next

- Stage 11 (next session): start phase C.3 — Goal 3 (flow-programmer). Per the job spec, flow definitions live in PG (`flows_definitions` dimension) with cross-instance NOTIFY; verbs need the same `Reversible` snapshot-before-write contract.

## What you need to know

- The integration test uses the in-memory `InMemoryChWriter`; the production `starter-store-clickhouse::ChClient`-backed `ChWriter` impl lands in a later phase (noted in the test module header).
- Only `retention.set` is exercised end-to-end here; `rule.write` and `mart.create` are still covered by their per-verb unit tests in `rubix-tools` (stage 8/9). The doc lists all three.
- The skill/flow tool ids switched from underscore (`rubix.clickhouse.rule_write`) to dot (`rubix.clickhouse.rule.write`) to match the actual `ToolDefinition::name` values.

## Open questions

- (none)
