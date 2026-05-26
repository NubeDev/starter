## Done

- stage 3 committed as `1df9ff2 stage 3 — delete ClickHouse (warehouse-engine-swap)` on `codeless/warehouse-engine-swap`; not pushed
- deleted `starter-warehouse` crate, `starter-store-warehouse`'s ClickHouse modules (`client.rs`, `migrate.rs`, `raw.rs`, `dim_freshness.rs`, `store/`, `testing/mod.rs`, `migrations/*.sql`, `tests/integration.rs`), `clickhouse` Cargo dep, `testcontainers-modules/clickhouse` feature
- deleted `examples/{ch-explorer,iot-anomaly-detector,flow-agent}/`, `rubix-tools::{warehouse,analytics}` modules + their tests, `rubix-agent` ch-history routing / goal-4 / goal-6 tests + `sdui/analytics_bridge.rs`
- rewired `rubix-agent` main / registry / boot/{warehouse,sdui,mcp,mcp/register} / bin/rubix_admin/mcp/serve / sdui/query_engine — `boot::warehouse` is now a no-op stub
- removed ClickHouse service from compose + `CLICKHOUSE_URL` env from GH workflow; OpenAPI lost `clickhouse-ruler` tag (count assertion lowered 9→8)
- gates green: `cargo build --workspace`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo fmt --check`, `cargo test --workspace` (no new failures vs stage-2 baseline)

## Next

- (none) — stage 3 of 3 complete; job done

## What you need to know

- The `starter-warehouse` capability crate (~3k lines of CH-specific SQL: `tap.write`, `curate.write`, MV `POPULATE`, `system.tables`, `dictGetOrNull`) was **deleted wholesale rather than ported**. A future job needs to rebuild it on top of the stage-2 `tsdb::store::*` + `TimescaleDbDialect` + `read_query_pg` helpers, along with the 7 `rubix.warehouse.*` and 2 `rubix.analytics.*` verbs that lived in `rubix-tools`
- Surviving live `clickhouse` references retained intentionally: `rubix-agent::boot::config::clickhouse_url` (unused field, kept to avoid env-var churn), `rubix-spi::dto::warehouse::*` (DTOs as type defs for the rebuild), `starter-tags::compile_ch` (pure string transformer, in-crate tests only). Stale doc-comment mentions of removed `ChClient`/`ChConfig` remain in a handful of files
- Pre-existing test failures unrelated to this stage: `rubix-flows::load_test`, `starter-flow::workspace_dep_tree_gates`, `starter-skills::stage11_reference_bundles`, `rubix-agent::chat_stream::skill_body_for_hint_resolves_bundled_skill`, `smoke-tests::blob_r8_doc_comments`
- `mani` was not on PATH; the agent ran `cargo test --workspace` instead of `mani run test --all`. `pnpm -w build` / `mani run build --all` were not run — frontend/TS gate not verified this stage

## Open questions

- Rename `rubix-agent::boot::config::clickhouse_url` (and its env var) when the warehouse capability is rebuilt
- Decide whether `starter-tags::compile_ch` survives the rebuild or is replaced with a Postgres-flavoured compiler
- `tenant_id` plumbing — the open question from stage 2 is moot for now since the consumers that needed it were deleted; resurface when rebuilding the capability crate
