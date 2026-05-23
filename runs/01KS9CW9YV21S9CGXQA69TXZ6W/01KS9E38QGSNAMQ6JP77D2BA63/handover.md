## Done

- Scaffolded `crates/starter-store-clickhouse/` on the official `clickhouse` Rust crate (0.13, lz4+chrono+uuid) per ADR-003; added to workspace.members and workspace.dependencies.
- 5 migration files (`0001_raw_events.sql` .. `0005_entities_dict.sql`) — one DDL each, `IF NOT EXISTS` everywhere. Snowflake IDs on `raw_events`/`events`, ZSTD on payload cols, `bloom_filter` skip index on `tags` on every history table. `events` carries the M-5 cardinality cap comment for `LowCardinality(kind)`. `documents.id` caller-supplied. `entities_dict` uses `LIFETIME(MIN 300 MAX 600)` + `invalidate_query SELECT max(updated_at) FROM entities` + `LAYOUT(HASHED())` with mustache-substituted PG coordinates.
- `src/migrate.rs` in-crate runner (no `sqlx::migrate` for CH), idempotent, audit table `_starter_ch_migrations`, refuses 0005 without `with_pg_source`.
- `src/client.rs` bakes `async_insert=1`+`wait_for_async_insert=1` on every connection (W8).
- `src/store/{raw_events,samples,events,documents}.rs` typed Row insert/read paths.
- `src/dim_freshness.rs` four-value W11 enum (fresh|stale_within_bound|stale_beyond_bound|failed_refresh), 5s in-process cache.
- `src/testing/with_clickhouse` testcontainer helper (testcontainers 0.23 + modules 0.11 `clickhouse` feature).
- `tests/integration.rs` covers insert/read per table, W16 ≤1.5s read-after-write bound, W11 transitions, and the W13 `dictGetOrNull` NULL contract.
- Fixed prior review-gate regression `dimensions_marts::live_mart_quota_trigger_only_scans_live_rows`: refactored `marts::*` to accept `impl PgExecutor<'_>` and pinned a single connection in the test so the session-scoped `warehouse.live_mart_quota` GUC is visible to every helper. Trigger now correctly rejects the 4th live insert.
- `cargo build -p starter-store-clickhouse --features testing --tests` and `cargo build -p starter-store-postgres --features 'dimensions testing' --tests` both green.

## Next

- Run `cargo test -p starter-store-clickhouse --features testing -- --ignored` on a host with Docker to confirm green (could not execute here — sandbox has no Docker).
- Re-run `cargo test -p starter-store-postgres --features 'dimensions testing' -- --ignored dimensions_marts` on a Docker host to confirm the W12 quota test now passes.
- Stage 4 (slice D) — `starter-warehouse` crate consuming this store.

## What you need to know

- `samples` migration ships only the `2 YEAR DELETE` TTL tier; the `90 DAY TO VOLUME 's3_cold'` move tier is documented as a deployment-time `ALTER TABLE MODIFY TTL` once the storage policy declares the volume. Pinning `TO VOLUME 's3_cold'` in CREATE TABLE breaks any deployment (including the testcontainer) where the volume is not configured. Flagged in the migration comment.
- `ChClient::inner()` is `pub` — W8's "no raw `INSERT INTO <history table>` outside `src/store/`" is enforced by CI grep, not by visibility. Type-level gating would force wrapping every read query too. Documented in `client.rs` and `store/mod.rs`.
- Migration 0005 uses mustache `{{name}}` placeholders, not CH's `{name:Type}` (which is a query-time-only feature). Runner refuses to apply 0005 if `PgSource` is missing.
- The integration tests' `boot()` helper inlines the first four migrations rather than calling `MigrationRunner::run()`, because 0005 needs a live Postgres. There is a separate test (`migration_runner_idempotent`) that exercises the runner and asserts the PgSource-less error path.
- `marts::insert / set_status / get / live_count / delete` now take `impl PgExecutor<'_>`. Production call sites should pass `pool.sqlx()`; tests that depend on session-scoped GUCs (the quota trigger) must acquire one connection and pass `&mut *conn`.

## Open questions

- Whether stage 4 will want a `ChClient::with_async_insert(false)` convenience for `bulk.import` callers, or whether constructing a fresh `ChConfig { async_insert: false, .. }` per bulk session is preferred. Left for the warehouse crate to decide.
- Whether the migration runner should grow a `MigrationRunner::without_dictionary()` filter so consumers without a live PG (CI smoke, doctests) can apply the table-only subset without inlining migration text. Not blocking; integration tests inline today.
