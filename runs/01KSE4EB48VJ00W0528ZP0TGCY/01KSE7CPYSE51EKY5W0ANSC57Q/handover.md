## Done

- Extended `FlowListItem` (Rust DTO in `rubix-spi/src/dto/flow_ops/list.rs` + TS in `rubix-client-ts/src/endpoints/flow_ops.ts`) with `body_yaml: String`; `rubix-tools/src/flow_ops/list.rs` populates it from the same `FlowRevisionRow` (no extra round-trip).
- Added new `rubix.flow_ops.kinds` verb: DTOs in `rubix-spi/src/dto/flow_ops/kinds.rs`, tool in `rubix-tools/src/flow_ops/kinds.rs` (`FlowKindsTool::from_behaviors` snapshots `kind_id`, `config_schema()`, derived title-cased `default_label`).
- Wired `FlowKindsTool` into `rubix-agent`'s `build_tool_registry` with built-ins `counter`, `log`, `trigger_schedule`; added `rubix.flow_ops.kinds` to the registry's quartet test (now quintet).
- Added `useFlowKinds()` in `rubix-client-react/src/hooks/flow-ops.ts` caching under `['rubix','flow_ops','kinds']`.
- Sibling tests: added `body_yaml_is_returned_inline_on_every_row`, `default_label_title_cases_last_segment`, `empty_registry_lists_zero_kinds`, `kinds_come_back_sorted_by_kind_id`, `from_behaviors_snapshots_kind_id_schema_and_label` (Rust); updated flowList body + added flowKinds test in TS; added useFlowKinds test in React.
- All targeted cargo + pnpm test suites green (`cargo test -p rubix-tools --lib flow_ops`: 21 passed; `cargo test -p rubix-agent --lib registry`: 8 passed; pnpm rubix-client-ts: 34 passed; pnpm rubix-client-react: 61 passed).
- Committed as `7e18919` "phase E.1 — flow_ops.list returns body_yaml + flow_ops.kinds endpoint".

## Next

- (none — fresh session picks up the next stage of 16)

## What you need to know

- Added `starter-flow-spi` as a regular dep of `rubix-tools` (was dev-only) and bumped the rubix-tools dev `starter-flow-nodes` features to include `counter` for the `from_behaviors` test.
- Added `starter-flow-nodes` as a main dep of `rubix-agent` with features `counter, log, trigger-schedule` so `builtin_kind_behaviors()` can construct seed instances. `TriggerExplicit::new()` requires a `TriggerChannelRegistry` so it's intentionally not seeded yet.
- `default_label` is a pure-presentation fallback derived in the tool (last reverse-DNS segment, underscores/hyphens → spaces, title-cased); no new field on `NodeBehavior` trait.
- The placeholder `useFlowDefinition` in the React hook still uses the synthetic-graph path; converting it to `yaml.parse(body_yaml)` is downstream stage work, not E.1.

## Open questions

- (none)
