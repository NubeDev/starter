# Proposal: Swap the Warehouse Engine (ClickHouse → TimescaleDB)

**Status:** Proposed
**Date:** 2026-05-26
**Author:** NubeDev

## Pre-production caveat

This system is **not in production**. There are no real tenants, no historical data worth preserving, and no operators to migrate. That removes a large chunk of the work this proposal would otherwise carry:

- No dual-write, no reconciliation, no cutover window.
- No L1/L2/L3 history backfill — drop and recreate.
- No deprecated verb aliases — old `rubix.clickhouse.*` names are removed, not forwarded.
- No release-window deprecation cycle for renames.
- `mart.create` undo data-loss caveats are immaterial against an empty store.
- Benchmarks are still worth running for forward-looking sizing, but not gating cutover.

The proposal below keeps the design intact but strips the migration scaffolding. **Hard delete is fine.**

## Summary

Swap the time-series engine behind the warehouse from ClickHouse to TimescaleDB, consolidating all storage onto the existing PostgreSQL instance. This eliminates a second database, leverages infrastructure already in operation, and preserves the full warehouse design (L1→L3 layers, mart rules, retention policies, tenancy model).

A second, parallel goal is to **rename the surface to be backend-neutral**. The verbs, traits, crates, and skills currently named `clickhouse*` become `warehouse*`. This is done first, as a pure rename, so the engine swap is not bundled with a vendor-name churn PR.

## Motivation

The system currently maintains two databases:

- **PostgreSQL** — dimensions, auth, config, undo changelog, and all relational data
- **ClickHouse** — raw events (L1), curated samples/events/documents (L2), and narrow mart tables (L3)

This split was a deliberate early choice, but it carries ongoing costs:

- Two connection pools, two sets of credentials, two migration runners
- The `entities_dict` materialized PostgreSQL dictionary in ClickHouse exists solely to bridge the gap — it refreshes on a timer and is a known lag source
- Two databases to operate, monitor, back up, and pay for
- Joins between relational data (Postgres) and time-series data (ClickHouse) are impossible at query time; mart rules work around this with pre-joined snapshots
- ClickHouse-specific async-insert discipline (`async_insert=1, wait_for_async_insert=1`) is a hidden invariant that breaks silently if a new write path skips it
- The verb namespace is vendor-named (`rubix.clickhouse.*`) — any engine swap forces an API rename, and the current names lie about what the surface is for (warehouse operations, not ClickHouse-specific operations)

TimescaleDB is a PostgreSQL extension. It adds hypertables, continuous aggregates, compression policies, and retention policies on top of standard PostgreSQL — same wire protocol, same client libraries, same migrations tooling already in use.

## Naming: vendor-neutral surface

The current surface leaks the engine name everywhere. Before any engine work, the surface is renamed to **`warehouse`** — which matches how this subsystem is already described in the codebase ("warehouse design", "L1→L3 warehouse layers").

| Current (vendor-named) | New (neutral) |
|---|---|
| `rubix.clickhouse.rule.write` | `rubix.warehouse.rule.write` |
| `rubix.clickhouse.mart.create` | `rubix.warehouse.mart.create` |
| `rubix.clickhouse.retention.set` | `rubix.warehouse.retention.set` |
| `ChWriter` trait | `WarehouseWriter` |
| `ChRuleReversible` / `ChMartReversible` / `ChRetentionReversible` | `WarehouseRuleReversible` / `WarehouseMartReversible` / `WarehouseRetentionReversible` |
| `starter-store-clickhouse` crate | `starter-store-warehouse` (engine-specific impls live behind it) |
| `clickhouse-ruler` skill / flow YAML | `warehouse-ruler` |
| `rubix_tools::clickhouse::mart` | `rubix_tools::warehouse::mart` |
| `clickhouse-ruler.yaml` flow | `warehouse-ruler.yaml` |

Old verb names are **removed outright** — no deprecated aliases, no forwarding shim. Pre-production, so there are no external callers to protect.

Alternatives considered and rejected: `timeseries` (still vendor-shaped, and marts are aggregated tables, not strictly time series); `analytics` (already in use by `rubix.analytics.report`); `olap` (jargon, not how the codebase describes the subsystem).

## What changes

### What stays the same

- The L1→L2→L3 layering model and the reads-default-to-L3 rule
- Per-row `tenant_id` tenancy (ADR-003 is unchanged)
- The semantics of the warehouse verbs (only their names and underlying SQL change)
- The `WarehouseWriter` trait seam and the snapshot-before-write contract
- The undo/reversible machinery
- The `warehouse-ruler.yaml` flow shape — only the underlying tool implementations change
- Idempotence guarantees on `mart.create` and `retention.set`

### What is replaced

| ClickHouse concept | TimescaleDB equivalent |
|---|---|
| `MergeTree` partitioned by `toYYYYMM(...)` | `hypertable` with `chunk_time_interval` (sized per table) |
| `toStartOfInterval(ts, INTERVAL N SECOND)` | `time_bucket(INTERVAL '...', ts)` |
| `LowCardinality(String)` | dropped — Postgres `TEXT` with indexes |
| `Map(String,String)` tags column with bloom filter | `jsonb` column with GIN index (selectivity differs — see Risks) |
| `ZSTD(3)` per-column compression | TimescaleDB columnar chunk compression |
| TTL clause (`toDateTime(...) + INTERVAL N DAY`) | `add_retention_policy(table, INTERVAL '...')` |
| `ALTER TABLE MODIFY TTL` | `remove_retention_policy` + `add_retention_policy` |
| `SHOW CREATE TABLE` for snapshots | `pg_dump -t` for tables; `pg_get_viewdef` + `timescaledb_information.continuous_aggregates` for caggs |
| `entities_dict` materialized PG dictionary | eliminated — direct JOIN in the same DB |
| `async_insert=1` write discipline | `COPY` or batch `INSERT` via sqlx |
| `MartSpec` DDL → `CREATE TABLE ... ENGINE=MergeTree` | `MartSpec` DDL → `CREATE TABLE` + `create_hypertable` (or a continuous aggregate, below) |
| Mart aggregation via scheduled `INSERT INTO ... SELECT` | TimescaleDB **continuous aggregates** with a refresh policy |

### Chunk sizing

`chunk_time_interval` is the TimescaleDB analogue of partition granularity. Wrong sizing is expensive in both directions — too small means chunk explosion and planner overhead; too large hurts compression and retention drop performance. This is a per-table decision and must be picked in Phase 1 before any write paths land. Starting point: match the current effective ClickHouse partition cadence (monthly for L1, weekly for L2 marts).

### The mart / continuous aggregate translation

This is the most significant porting work. A `MartSpec` currently generates a `CREATE TABLE` DDL plus a periodic `INSERT INTO mart SELECT ... GROUP BY time_bucket, group_by_cols` query run by the ruler flow.

In TimescaleDB this maps to a **continuous aggregate**:

```sql
CREATE MATERIALIZED VIEW mart_name
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('60 seconds', ts) AS bucket,
    tenant_id,
    <group_by_cols>,
    sum(value)    AS value_sum,
    max(value)    AS value_max,
    avg(value)    AS value_avg,
    count(*)      AS value_count
FROM l2_curated_table
GROUP BY bucket, tenant_id, <group_by_cols>
WITH NO DATA;

SELECT add_continuous_aggregate_policy('mart_name',
    start_offset => INTERVAL '3 days',
    end_offset   => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');
```

Constraints and caveats to be aware of:

- Continuous aggregates cannot reference subqueries or CTEs in the `SELECT`.
- `UNION ALL` across source tables is not supported — one source hypertable per aggregate.
- **Refresh window is bounded.** Late-arriving data with `ts` older than `start_offset` is silently *not* materialized until a manual `CALL refresh_continuous_aggregate(...)`. This is a real semantic change from today's open-ended `INSERT INTO ... SELECT` — audit current mart sources for late-arrival tolerance.
- **Compressed chunks are write-restricted.** Once L1/L2 chunks are compressed by policy, `INSERT`/`UPDATE`/`DELETE` into them requires decompression first. Recent Timescale versions relax this for inserts but not fully. Backfill paths must account for this.
- Re-defining a continuous aggregate is destructive: `DROP MATERIALIZED VIEW` drops the underlying materialization hypertable as well. Same shape as today's `mart.create` undo caveat, but the mechanism is different and should be re-documented, not hand-waved as identical.
- `quantile` aggregations require `percentile_cont` or `timescaledb_toolkit`.
- **Tenancy on caggs:** `tenant_id` must appear in the `GROUP BY` of every cagg, and RLS on continuous aggregates requires `security_invoker = true` on the materialized view. Resolver-layer filtering remains the primary enforcement point (ADR-003).

The `MartSpec` DDL generation layer (`rubix_tools::warehouse::mart` after the rename) is rewritten to emit this shape. The reversible snapshot changes from `SHOW CREATE TABLE` to capturing the view definition from `timescaledb_information.continuous_aggregates` joined with `pg_get_viewdef`.

### Retention

`rubix.warehouse.retention.set` issues `ALTER TABLE MODIFY TTL` today (against ClickHouse). The TimescaleDB equivalent:

```sql
-- set
SELECT add_retention_policy('table_name', INTERVAL 'N days');

-- remove
SELECT remove_retention_policy('table_name');

-- current value (for snapshot)
SELECT config FROM timescaledb_information.jobs
WHERE proc_name = 'policy_retention' AND hypertable_name = 'table_name';
```

The `WarehouseRetentionReversible` snapshot shape (`{ table_name, days }`) is unchanged — only the SQL issued by the verb changes.

## Migration plan

Pre-production, so the plan collapses to three phases. No dual-write, no backfill, no cutover window — drop the ClickHouse service, drop its tables, rebuild empty on TimescaleDB.

### Phase 1 — Rename + decouple

1. Rename traits, crates, modules, verb names, skill, and flow YAML per the table above. Old names are deleted, not aliased.
2. Extract `MartSpec` DDL generation behind a `DdlDialect` trait with a `ClickHouse` impl. Makes Phase 2 additive rather than a rewrite.
3. Decide `chunk_time_interval` per hypertable (L1, L2 samples/events/documents) and bake values into the migration.
4. Swap the dev compose stack: TimescaleDB + `timescaledb_toolkit` replaces ClickHouse. No parallel running.
5. Audit existing `MartSpec` definitions against the cagg constraints (no subqueries/CTEs, single source, late-arrival window). Reshape any that won't translate.

### Phase 2 — TimescaleDB implementation

1. Wire `starter-store-warehouse` to a sqlx PgPool backend.
2. Implement `WarehouseWriter` against TimescaleDB: write paths for `raw_events`, `samples`, `events`, `documents` via `COPY` into hypertables.
3. Add `DdlDialect::TimescaleDb` impl for `MartSpec` → continuous aggregate DDL.
4. Port retention verb to `add_retention_policy` / `remove_retention_policy`.
5. Port rule verb snapshot to `timescaledb_information.continuous_aggregates` + `pg_get_viewdef`.
6. Replace `entities_dict` dictionary references in mart queries with direct JOINs.

### Phase 3 — Delete ClickHouse

1. Remove `starter-store-warehouse`'s ClickHouse dialect and any ClickHouse-specific impls.
2. Remove the ClickHouse docker service, env vars, migration runner, and CI matrix entries.
3. Remove the `entities_dict` materialized dictionary and its refresh job.
4. Hard delete — no data preservation, no rollback path needed.

## Benchmarks (informational, not gating)

No cutover to gate against — but worth measuring once on a synthetic load to set forward-looking expectations:

- L1 ingest throughput (events/sec) at expected production batch size
- L1 write p99 latency under sustained load
- Mart query p95 latency on representative cagg shapes
- Compression ratio on a week of synthetic L1 data

Numbers inform chunk sizing and capacity planning, not go/no-go.

## What does not change for operators

- The undo / changelog / `rubix.undo.last` path is unchanged
- Tenancy filters remain enforced at the resolver layer
- Retention tiers (L1: days–weeks, L2: months, L3: years) are unchanged
- The CI lint banning raw `INSERT`/`SELECT` outside typed paths carries forward

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Continuous aggregate constraints break existing `MartSpec` definitions | Phase 1 audit; flag any that use subqueries, CTEs, or multi-source joins before Phase 2 |
| Late-arriving data falls outside cagg `start_offset` and is silently not materialized | Audit each mart's late-arrival tolerance; widen `start_offset` or schedule manual refresh jobs for affected marts |
| Compressed chunks block backfill / corrections | Defer — no historical data; will need a decompress→edit→recompress runbook before going to production |
| `quantile` aggregations need `timescaledb_toolkit` | Add toolkit to the compose stack in Phase 1 |
| `Map`/bloom-filter → `jsonb`/GIN selectivity differs on high-cardinality tags | Promote hot tag keys into columns if GIN proves insufficient under real load (post-launch) |
| `mart.create` undo on a cagg drops the materialization hypertable | Document the caveat; immaterial pre-production |

## Long-term wins

- One database to operate
- Joins between entity dimensions and time-series marts work natively
- Standard PostgreSQL tooling (pgAdmin, `psql`, `pg_dump`, existing migration runner) covers everything
- No hidden async-insert discipline; writes are standard SQL
- `timescaledb_toolkit` provides `time_weight`, `lttb` downsampling, `candlestick_agg`, and statistical aggregates not available in ClickHouse without custom functions
- Vendor-neutral surface — a future engine swap (DuckDB, Citus, something else) is a `DdlDialect` impl, not an API rename
