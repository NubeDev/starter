## Done

- migration `rubix/crates/rubix-agent/migrations/0002_history/{up,down}.sql` for `system_disk_history` (tenant_id UUID, host, percent_used, free_bytes, epoch_ms; PARTITION BY toYYYYMM(toDateTime(epoch_ms/1000)); ORDER BY tenant_id,host,epoch_ms)
- extended `starter-store-clickhouse::MigrationRunner` with `with_extra_migration(name, sql)`; new `#[ignore = "requires docker"]` test in `crates/starter-store-clickhouse/tests/integration.rs` covers it
- new `rubix/crates/rubix-agent/src/boot/clickhouse.rs` (`apply_ch_migrations`) applies `rubix/0002_history/up.sql` through the shared runner behind `RUBIX_CH_URL`; wired into `main.rs` boot log
- `rubix-tools::system::disk::DiskTool` now optionally carries an `Arc<ChClient>` + `tenant_id` + `host`; `invoke()` calls `write_history` then `run_insights_gate`
- `run_insights_gate` holds the literal `// TODO(upstream: rule.rhai migration) — promote to starter-insights::RuleRegistry once a second rule appears.\nif response.percent_used > 90 { alert_send::dispatch(...).await? }`
- `rubix-tools::system::alert_send::dispatch(severity, Diagnostic)` + process-wide `dispatched_count()` for assertion deltas; logs severity+key+params via tracing
- new test `rubix/crates/rubix-tools/tests/system_disk_insights_test.rs` proves the gate fires exactly once at 95% and zero at 50%
- two new unit tests in `disk.rs::tests` pin the `system_disk_history` insert SQL (tenant_id reaches the row as `toUUID('...')`; single quotes in host are doubled)
- `crates/starter-store-clickhouse/src/lib.rs` re-exports `pub use clickhouse;` so consumers reach `Row` derive without a direct dep
- `docs/design/warehouse/README.md` rewritten present-tense; commits to per-row `tenant_id` as v0 with rationale + revisit trigger ("a single tenant's read volume forces per-tenant tables")
- `docs/design/insights/README.md` rewritten present-tense; explains the hardcoded `if` and promotion trigger to `rule.rhai`
- `cargo test -p rubix-tools` and `cargo test -p rubix-agent --lib boot` both green; `./rubix/scripts/lint-doc-refs.sh` clean; `cargo tree -p rubix-agent --invert clickhouse` shows only the transitive path through starter-store-clickhouse
- committed as `90facb5` on `codeless/rubix-thin-slice-v2`

## Next

- stage 3 (block 3, PR 5): REST handler + `rubix-admin system disk [--json]` parity reaching the same `probe()` in-process

## What you need to know

- The exit-signal "a probe() call writes exactly one row" is verified at the **SQL-shape level** by unit tests (`history_insert_sql_*`) rather than against a live container in `rubix-tools`. Writing a typed-row container test from rubix-tools requires the `clickhouse::Row` derive, and the derive hard-codes the absolute path `::clickhouse::` — so any rubix-side typed-Row test would need a direct `clickhouse` dep, which violates the stage's "no direct clickhouse dep to any rubix crate" rule. The end-to-end container coverage lives in `starter-store-clickhouse`'s own integration suite (`migration_runner_applies_consumer_supplied_extras`); the production boot exercises the same SQL via `apply_ch_migrations`.
- `DiskTool` is no longer a unit struct — every caller must use `DiskTool::default()`. `registry.rs`, `system_disk_test.rs` and `system_disk_recorded_llm_test.rs` were updated.
- The `clickhouse` crate is re-exported from `starter-store-clickhouse` (`use starter_store_clickhouse::clickhouse;`). Use this path from rubix code if you need typed rows or `Compression`; do not add `clickhouse` as a direct dep.
- The insights gate uses the literal `> 90` (the spec) and also defines `pub const INSIGHTS_DISK_ALERT_THRESHOLD: u8 = 90` so the threshold has a name for the future RuleRegistry migration; a `debug_assert_eq!` keeps them in sync.

## Open questions

- (none — T3/T4 were pre-answered in SCOPE.md and the stage decisions follow them)
