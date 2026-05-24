## Done

- Filled rubix-tools/src/clickhouse/{rule_write,mart_create,retention_set}.rs as concrete `Tool` + `ReversibleTool` impls backed by a new `ChWriter` trait in clickhouse/store.rs (with `InMemoryChWriter` for tests). Each verb snapshots prior state before the write.
- mart.create's empty-snapshot case routes to `restore_mart(snap{ddl:None})` which the production impl maps to `DROP TABLE IF EXISTS`; data-loss caveat surfaced in the response docstring, the `rubix.clickhouse.mart.created` catalogue message, and the design-doc link.
- Added six MessageKeys to en.json + es.json: `rubix.clickhouse.rule.{written,invalid}`, `rubix.clickhouse.mart.{created,already_exists}`, `rubix.clickhouse.retention.{set,unchanged}`.
- Flesh-filled the three placeholder DTO files in rubix-spi/src/dto/clickhouse/ with request/response structs, REQUIRED_PERMISSION, and the five-field ToolDescriptor.
- 11 unit tests pass (`cargo test -p rubix-tools --lib clickhouse::`); `cargo build -p rubix-agent` green; `scripts/lint-doc-refs.sh` clean.
- Committed as `stage 9: phase C.1 — Goal 4 verbs — feat(rubix-tools) clickhouse-ruler verbs` (b16b487).

## Next

- Stage 10 (phase C.2 expected): wire ChWriter to a real `ChClient`-backed impl, register the three Reversible kinds + verbs at agent boot, fill the clickhouse-ruler skill + flow YAML allowed_tools, add the goal-4 integration test, and author `docs/design/clickhouse-rules/README.md` (already referenced by the new code).

## What you need to know

- The three Reversible impls (`ChRuleReversible`, `ChMartReversible`, `ChRetentionReversible`) are exported from `clickhouse::store`; resource-kind constants are `clickhouse_rule`, `clickhouse_mart`, `clickhouse_retention` — already on the `undo_snapshots` CHECK list from stage A.2.
- `mart.create` records `Op::Create` with `before = ChMartSnapshot{ ddl: None }` on a fresh create, which is what drives the `DROP TABLE IF EXISTS` inverse. Idempotent re-create returns `was_already_present = true` and emits NO `ChangeDraft` (mirrors the user.disable pattern).
- `retention.set` with `days = 0` clears the TTL; the response carries `prior_days` from the snapshot probe so integration tests can assert it without re-querying `system.tables`.
- `validate_ddl` for rule.write refuses anything that doesn't start with `CREATE ` or `ALTER ` (case-insensitive after trim); the error message embeds the `rubix.clickhouse.rule.invalid` MessageKey so the agent loop's i18n layer renders it correctly.
- DTO files docstring-link `docs/design/clickhouse-rules/README.md` — the file doesn't exist yet; the lint script only forbids the high-tier doc refs so this is fine until Phase C closes.

## Open questions

- None blocking this stage. SCOPE Open Question 3 (snapshot JSON shape per `resource_kind`) is now answered for Goal 4 via the `ChRuleSnapshot` / `ChMartSnapshot` / `ChRetentionSnapshot` structs.
