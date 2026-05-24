## Done

- Implemented `AnalyticsQueryTool` in `rubix/crates/rubix-tools/src/analytics/query.rs` (named-template lookup via `include_dir!`, CH `{name:Type}` param binding through `query.param()`, `JSONEachRow` row parsing).
- Filled `rubix-spi::dto::analytics::query` with `AnalyticsQueryRequest`/`Response` DTOs + five-field `DESCRIPTOR`.
- Shipped 6 SQL templates under `rubix-tools/src/analytics/templates/`: disk_history_weekly, alert_count_weekly, flow_run_summary_weekly, user_activity_weekly, clickhouse_writes_weekly, undo_count_weekly.
- Added 3 MessageKeys (`rubix.analytics.query.ran`, `.unknown_template`, `.bind_error`) to EN + ES catalogues in the same commit.
- Added `include_dir = "0.7"` to rubix-tools deps; added `starter-store-clickhouse` testing-feature dev-dep.
- Wrote `tests/analytics_query_test.rs` — testcontainers CH, seeds synthetic tables, asserts every template runs via `Tool::invoke`; unit tests cover the closed catalogue + unknown-template error path. All non-ignored tests pass.
- Committed as `feat(rubix-tools) analytics.query + 6 named templates`.

## Next

- Stage 9 (per WORKFLOW): analytics.report verb — render via starter-export, persist via starter-blob-fs.

## What you need to know

- Templates read from CH tables that don't all exist in production yet (`changelog`, `flow_run_history`); the integration test creates minimal schemas for them. Real warehouse migrations land in a later stage.
- Param binding uses `clickhouse::Query::param()`; values are `serde_json::Value` and the driver serializes via its own Serialize impl, so callers reference slots as `{name:Type}` in template SQL.
- `bind_error` takes `impl Display` so the rubix-tools crate avoids a direct `clickhouse` dep — driver errors pass through `ChClient::inner()` only.
- Integration test is `#[ignore]` (Docker-required), same pattern as other rubix-tools CH integration tests.

## Open questions

- (none)
