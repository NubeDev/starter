## Done

- Stage 1 of warehouse-engine-swap implemented and committed as `53ac6b4 stage 1 — rename + decouple (warehouse-engine-swap)` on branch `codeless/warehouse-engine-swap`.
- Crate `starter-store-clickhouse` → `starter-store-warehouse`; engine-specific impls remain inside.
- Rubix module renames: `rubix_tools::clickhouse` → `::warehouse`, `rubix_spi::dto::clickhouse` → `::warehouse`, `rubix_client::clickhouse` → `::warehouse`, `rubix-agent::boot::clickhouse` → `::warehouse`. `ch_client_writer.rs` → `warehouse_client_writer.rs`.
- Trait/type/constant renames: `ChWriter→WarehouseWriter`, `InMemoryChWriter→InMemoryWarehouseWriter`, `Ch{Rule,Mart,Retention}{Snapshot,Reversible}→Warehouse*`, `ChTableSummary→WarehouseTableSummary`, `CH_*_KIND→WAREHOUSE_*_KIND` (values `warehouse_rule|mart|retention`).
- Verb namespace `rubix.clickhouse.*` → `rubix.warehouse.*` across Rust dispatch, OpenAPI, TS client, React hooks (`useClickhouse*→useWarehouse*`), frontend admin panels, catalogues, skill, flow YAML.
- Skill + flow: `clickhouse-ruler` → `warehouse-ruler` (dirs, ids, bodies). Tests renamed accordingly.
- `DdlDialect` trait added at `crates/starter-warehouse/src/ddl/dialect.rs` with `ClickHouseDialect` impl producing byte-identical legacy output (unit test asserts).
- Chunk-interval constants in `crates/starter-store-warehouse/src/chunk_intervals.rs`: `L1_CHUNK_INTERVAL="1 month"`, `L2_CHUNK_INTERVAL="1 week"` (+ per-table aliases).
- Dev compose swapped to `timescale/timescaledb-ha:pg16`; rubix port 5434 to avoid 5433 collision; `wait-for-deps.sh` updated.
- Mart audit at `rubix/docs/proposal/stage1-mart-audit.md`: 6 sites, all clean; one Stage-2 note re `stddevPop` needing `timescaledb_toolkit::stats_agg`.
- Pre-existing bugs fixed in passing: 3 `build_tool_registry` test callers, stale analytics template list test, MSRV `is_none_or` in `starter-tool-sysdiag`.
- Verification: `cargo build --workspace`, `cargo clippy --workspace --all-features --tests -- -D warnings`, `cargo fmt --all -- --check`, `mani run build --all`, `cargo test --workspace --no-run`, `cargo test -p rubix-tools --lib` (155/155) — all green.

## Next

- Stage 2 (separate session) — sqlx PgPool backend for `starter-store-warehouse`, `WarehouseWriter` impl over TimescaleDB (`COPY` paths for raw_events/samples/events/documents), `TimescaleDbDialect` for `MartSpec` → continuous aggregate DDL, retention verb on `add_retention_policy`/`remove_retention_policy`, snapshot via `timescaledb_information.continuous_aggregates` + `pg_get_viewdef`, replace `entities_dict` references with direct JOINs.

## What you need to know

- Existing tests still run against the ClickHouse dialect — Stage 2 is gated behind `DdlDialect`, additive not rewrite.
- `starter-store-warehouse`'s `testing` feature still pulls the ClickHouse testcontainers module — Stage 2 swaps it.
- Docker compose for ClickHouse renamed to `docker/docker-compose.warehouse.yml` and now declares TimescaleDB; CI workflow refs updated.
- Old `packages/starter-ui-ch-explorer` renamed to `starter-ui-warehouse-explorer-legacy` because a non-legacy `starter-ui-warehouse-explorer` already existed.
- Historical `docs/sessions/*` journals intentionally left untouched.

## Open questions

- (none)
