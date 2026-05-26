## Done

- Added `starter-store-warehouse::tsdb` — `WarehouseClient` over `sqlx::PgPool`, inline migrations creating hypertables sized per `chunk_intervals` (L1 monthly, L2 weekly) with explicit `tenant_id` columns, and `COPY`-driven typed writers for `raw_events` / `samples` / `events` / `documents`.
- Added retention helpers (`add_retention_policy` / `remove_retention_policy` / `snapshot_days` reading `timescaledb_information.jobs WHERE proc_name='policy_retention'`, parsing both `P30D` and `30 days` forms) with unit tests.
- Added cagg helpers — snapshot via `timescaledb_information.continuous_aggregates` + `pg_get_viewdef`, plus a `CALL refresh_continuous_aggregate` driver.
- Added `tsdb::testing::with_timescale()` testcontainer factory on `timescale/timescaledb-ha:pg16`.
- Added `starter-warehouse::ddl::TimescaleDbDialect` emitting `CREATE MATERIALIZED VIEW … WITH (timescaledb.continuous, security_invoker = true) … WITH NO DATA` + the proposal's standard `add_continuous_aggregate_policy(3 days / 1 min / 1 min)` shape. `tenant_id` is forced into every cagg's GROUP BY (ADR-003). `quantile` aggregations map to `percentile_cont`.
- Added `starter-warehouse::ddl::mart::read_query_pg` doing a direct `LEFT JOIN entities` (replaces `dictGetOrNull('entities_dict', …)`).
- Added `crates/starter-warehouse/tests/tsdb_smoke.rs` — full ingest → cagg → query → retention round trip (`#[ignore]`, requires Docker).
- `cargo build --workspace`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo fmt --check` are all green. Existing ClickHouse paths and tests untouched.
- Committed as `stage 2 — TimescaleDB implementation (warehouse-engine-swap)`.

## Next

- Stage 3: delete the ClickHouse path — `ChClient` / `ChConfig` / `ChClientError`, `crates/starter-store-warehouse/src/{client,migrate,raw,store,testing}.rs`, the ClickHouse `.sql` migration files including `0005_entities_dict.sql` and any timer-based `entities_dict` refresh / lag observers, the `clickhouse` Cargo dep + `testcontainers-modules/clickhouse` feature, `ClickHouseDialect` and the byte-for-byte regression test, plus every downstream call site (`rubix-tools::warehouse::*`, `rubix-agent`, `examples/{ch-explorer,iot-anomaly-detector,flow-agent}`, `starter-warehouse::explorer`).
- Stage 3: rewire those call sites onto `WarehouseClient` / `tsdb::store::*` / `TimescaleDbDialect` / `read_query_pg`, and port the warehouse REST/MCP verbs (`rubix.warehouse.{rule.write,mart.create,retention.set}`) to the new helpers.
- Stage 3: drop the ClickHouse service / env vars / migration runner / CI matrix entries and the `entities_dict` refresh job. The dev compose `docker/docker-compose.warehouse.yml` is already on TimescaleDB.

## What you need to know

- This stage is **additive** by design — the ClickHouse paths still compile and the legacy test suite still passes. Stage 3 is where the deletion happens.
- `MartDdl` is reused as the carrier for the cagg dialect: `create_view` = the cagg `CREATE`, `create_target` = the `add_continuous_aggregate_policy` call, `drop_view` = `DROP MATERIALIZED VIEW … CASCADE`, `drop_target` = `remove_continuous_aggregate_policy`. The verb consumer must execute `create_view` BEFORE `create_target` (reverse of the ClickHouse ordering). `target_name == view_name` on this dialect.
- New hypertables include a `tenant_id TEXT NOT NULL` column on every table. The ClickHouse schema did not; downstream writers will need to start supplying it. Smoke test seeds an `entities` table with `(id, tenant_id, display)` because the new mart read query joins on `(id, tenant_id)`.
- `tsdb::retention::add_retention_policy` removes any existing policy first to make set-then-set idempotent (Timescale's `add_retention_policy` errors on conflict even with `if_not_exists`).
- The smoke test is `#[ignore]` and gated `#[cfg(all(feature = "warehouse", feature = "testing"))]`; not run in default CI. Invoke with `cargo test -p starter-warehouse --features warehouse,testing --test tsdb_smoke -- --ignored`.
- `sqlx` was added to `starter-store-warehouse` with `postgres,macros,migrate,chrono,json,uuid` features and `futures = "0.3"` (needed for the COPY sink). `testcontainers-modules` gained the `postgres` feature flag.

## Open questions

- The proposal mentions `timescaledb_toolkit` for advanced aggregates; the dialect currently inlines `percentile_cont(0.95)` for `quantile`. A future patch should plumb the percentile through `AggregationSpec` rather than hard-coding 0.95.
- The TimescaleDB hypertable schemas added `tenant_id` but the legacy ClickHouse schemas did not. Stage 3 needs to decide whether the rubix-tools writers (`rubix_tools::warehouse::warehouse_client_writer` et al.) plumb `tenant_id` through the existing flow node inputs or default it from session context.
