## Done

- Wired the seven `rubix.dashboard.*` verbs (get, list, create, update, duplicate, delete, page_set) into `rubix-agent/src/registry.rs` so they appear in the REST tools router and in the `tool_registry_snapshot` that `boot/mcp/register.rs` threads into `RubixAiAgentNode` — making them dispatchable by the dashboard-assistant flow's model loop (R7).
- Added public `InMemoryDashboardStore` in `rubix-tools/src/dashboard/store.rs` as the laptop/no-PG fallback, mirroring the PG insert-only `(tenant_id, page_id)` supersede contract.
- New registry unit test `registry_contains_every_dashboard_verb` asserts the seven names are present (passes alongside 41 existing tests).
- New integration test `rubix-agent/tests/dashboard_crud_test.rs` mirrors the goal-3 shape: testcontainers PG + PgDashboardStore + PgChangeRecorder + PgChangeLog + DashboardReversible, walking create → get → update → conflict-on-stale → undo.last → delete → duplicate → list-with-filter → page_set, gated with `#[ignore]` like `dashboards_definitions_test.rs`.
- `cargo test -p rubix-agent --test dashboard_crud_test` green (1 ignored).
- Committed as `cd4a18f`.

## Next

- Stage 12 of 16 per the scope-file dependency graph in `rubix/docs/scope/dashboards/README.md`.

## What you need to know

- The dashboard verbs in registry.rs are backed by `InMemoryDashboardStore` + a local `InMemoryGraphStore` + a local `StaticRegistry` — production swap to `PgDashboardStore` / the agent's live `Engine`-backed graph is a one-line change in this file (same pattern as the other in-memory adapter notes in the module docstring).
- `--ignored` invocation of the new test fails locally with `gen_random_uuid() does not exist`, but the pre-existing `dashboards_definitions_test.rs` has the identical failure — both rely on a Postgres image with pgcrypto/uuid-ossp available in the integration-job CI environment. Not a regression introduced here.
- `boot/mcp/register.rs` was not edited directly: it already consumes `registry::build_tool_registry` to populate `tool_registry_snapshot`, so registering the verbs in `registry.rs` is the single edit that surfaces them to MCP via the dashboard-assistant flow.

## Open questions

- (none)
