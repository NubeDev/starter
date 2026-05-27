# ADR-003 — Warehouse runs on ClickHouse; Postgres keeps OLTP

> 🪦 **RETIRED 2026-05-27** — superseded by
> [ADR-004 — TimescaleDB warehouse, Postgres OLTP](./ADR-004-timescaledb-warehouse.md)
> and implemented by the
> [`warehouse-engine-swap`](../../rubix/docs/proposal/warehouse-engine-swap.md)
> proposal. The ClickHouse engine was deleted in stage 3 of that
> proposal; the warehouse now runs on TimescaleDB inside the existing
> Postgres instance. ADR-002's "single database" thesis effectively
> won — see ADR-004 for the current decision. The rest of this
> document is kept verbatim as the historical record of why ClickHouse
> was chosen at the time. **Do not cite for current design — cite
> ADR-004.**

- **Status:** Retired (superseded by ADR-004)
- **Date:** 2026-05-23 (retired 2026-05-27)
- **Scope:** The `starter-warehouse` capability and any consumer
  (starting with [`examples/flow-agent`](../../examples/flow-agent))
  that needs a tag-driven analytics layer with dashboard-grade read
  latency and years-of-history retention.
- **Supersedes:** [ADR-002](./ADR-002-timescaledb-same-db.RETIRED.md)
  (TimescaleDB in the same Postgres). ADR-001 (`flow-agent` is
  Postgres-only for OLTP) is **unchanged** — this ADR is about the
  *analytics* layer, not the OLTP store.

## Context

The product needs to:

1. Ingest arbitrary incoming data (driver telemetry, webhook payloads,
   flow outputs) at thousands of rows/sec.
2. Curate it into typed tagged tables.
3. Roll it up into pre-aggregated marts that dashboards open in
   <50 ms regardless of how many years of history sit underneath.
4. Retain years of cold history at bounded cost (S3 tiering acceptable).
5. Let an AI agent compose pages by tag query and define new marts
   declaratively.

[ADR-002](./ADR-002-timescaledb-same-db.RETIRED.md) pinned this layer
to TimescaleDB inside the same Postgres database. That call was made
to keep "one database, full stop." It is now retired. The deciding
factors that flipped the call:

- **Cold-storage cost trajectory.** Years of history at bounded cost
  means S3 tiering. Timescale's tiered-storage story (TSL) is gated
  behind the commercial license; the Apache-2 lane (which ADR-002
  insisted on, see retired W8) does not include it. ClickHouse ships
  `TTL ... TO VOLUME 's3_cold'` in the Apache-2 OSS build.
- **Read latency at scale.** ClickHouse `AggregatingMergeTree` marts
  are kept current **per flushed block** (the MV fires synchronously
  on each block written to the source table, which under our
  `async_insert=1` regime is ≤1 s after the row write — see
  [Warehouse W16](../Warehouse/SCOPE.md#w16--read-after-write-boundary)
  for the read-after-write contract). Timescale continuous
  aggregates refresh on a policy (default minutes — bounded above by
  the refresh window, not by the block flush). At dashboard read
  time, ClickHouse is fresher *and* faster on the rollup path.
- **Compression.** ZSTD on cold parts typically gives 5–10× on
  telemetry-shaped data, often 10×+ with low-cardinality tag values.
- **Headroom.** A single ClickHouse node comfortably handles 10s of
  TB. The product's projected scale lives entirely inside single-node
  ClickHouse for the foreseeable future; the upgrade path (replication,
  sharding) exists if needed but is not on the critical path.

The single-DB constraint is dropped. The product now runs **two
databases on purpose**:

- **Postgres** owns everything OLTP — `flows`, `agents`, `runs`,
  `agent_sessions`, `skill_approvals`, users, sessions, plus the
  warehouse's *dimension* tables: `entities`, `entity_refs`,
  `tag_definitions`, the `marts` catalog. Anything with FKs,
  transactional writes, or frequent UPDATEs lives here.
- **ClickHouse** owns the *history* — `raw_events`, `samples`,
  `events`, `documents`, and every rollup materialized view
  (`mart_*`). Append-shaped, high-volume, queried by dashboards.

The two databases connect through a one-way seam: ClickHouse reads
Postgres via the `Dictionary(SOURCE(POSTGRESQL(...)))` mechanism to
resolve dimensions at query time. No replication, no CDC, no
two-phase commit.

## Decision

**The warehouse runs on ClickHouse. Postgres remains the OLTP store
and the source of truth for warehouse dimensions. The flow engine
writes history to ClickHouse with `async_insert=1`.**

Concretely:

- A new crate `starter-store-clickhouse` provides the warehouse store
  impls (entities are read from Postgres; samples/events/documents
  are written to ClickHouse). The existing `starter-store-postgres`
  retains all OLTP responsibilities and gains a small `dimensions`
  feature exporting the `entities` / `entity_refs` / `marts` /
  `tag_definitions` schema.
- A new migration namespace `crates/starter-store-clickhouse/migrations/`
  follows the workspace convention. Migrations target the ClickHouse
  HTTP endpoint via the official `clickhouse` Rust crate.
- All ClickHouse SQL is **Apache-2.0 OSS features only**. No
  ClickHouse-Cloud-specific features (SharedMergeTree, ClickPipes) are
  used. Replication via `ReplicatedMergeTree` + `clickhouse-keeper` is
  available but not on day one.
- Every flow-engine write to ClickHouse uses `async_insert=1,
  wait_for_async_insert=1`. The server buffers ~1 s / 1 MB / 450
  queries and flushes a single part. `tap.write` and `curate.write`
  write one row per call — this is intentional and safe under
  `async_insert=1`. **Row-at-a-time `INSERT` without `async_insert=1`
  is forbidden** by client-side enforcement in the store crate; that
  is the invariant, not row-at-a-time itself.
- Tags storage on ClickHouse history tables is `Map(String, String)`
  with a `bloom_filter` skip index. The Postgres dimension tables
  keep `JSONB` + GIN as today. See updated
  [Tags SCOPE T8](../Tags/SCOPE.md#t8--two-sql-compilation-targets-plus-an-in-process-matcher).

## Consequences

### Positive

- **<50 ms dashboard reads.** Pre-aggregated `AggregatingMergeTree`
  rollups, ordered by the dimensions dashboards filter on, hit this
  trivially on a single node with hot data on NVMe.
- **Per-flushed-block MV freshness.** ClickHouse incremental MVs
  fire on every block written to the source table. Under our
  `async_insert=1` regime, that flush happens ≤1 s after the row
  write; the MV then materialises into the target synchronously
  with the flush. There is no refresh policy to schedule, no
  backfill catch-up window, no missed chunks. See
  [Warehouse W16](../Warehouse/SCOPE.md#w16--read-after-write-boundary)
  for the read-after-write bound this implies.
- **Cold S3 tiering in OSS.** `TTL ts + INTERVAL 90 DAY TO VOLUME
  's3_cold'` moves old parts to object storage transparently.
  Apache-2.0, no commercial tier needed.
- **Headroom.** A single ClickHouse node handles the projected
  workload with orders of magnitude to spare. No cluster on day one.
- **FK integrity preserved where it matters.** `entity_refs` stays in
  Postgres with `PRIMARY KEY (from_id, rel, to_id)` and FKs to
  `entities(id)`. The warehouse's referential integrity story does
  not regress.
- **Tag-bag containment with index acceleration.** `Map(String, String)`
  + `bloom_filter` skip index covers the dominant tag query shape
  (equality containment, the ClickHouse equivalent of `jsonb @> '…'`).

### Negative

- **Two databases.** Two backup pipelines, two upgrade paths, two
  monitoring stacks, two on-call playbooks. This is the cost the
  product now accepts in exchange for purpose-built analytics. The
  seam between them is narrow (a Postgres-sourced Dictionary on
  ClickHouse) — but it is real.
- **No `sqlx` on the ClickHouse side.** ClickHouse's Postgres-wire
  compatibility is not sufficient for `sqlx`'s codegen and feature
  use. The warehouse store crate pins the official `clickhouse`
  Rust crate (HTTP + RowBinary) — chosen over `klickhouse` for
  closer alignment with ClickHouse's documented protocol and a
  smaller surface area. Two Rust DB ecosystems live in the
  workspace; the second one is bounded to one crate.
- **Insert discipline is non-optional.** ClickHouse hates
  row-at-a-time INSERTs. The store crate enforces `async_insert=1`
  on every write connection. A flow node that bypasses the store
  crate and writes raw rows can still create a "too many parts"
  incident; the linter catches direct `INSERT` strings in node code.
- **Dictionary refresh lag.** Entity dimensions on ClickHouse are
  read through a `LIFETIME(MIN 300 MAX 600)` dictionary, so they
  trail Postgres by **up to 10 minutes (configurable via
  `LIFETIME(MIN/MAX)`)**. `invalidate_query` polls `max(updated_at)`
  and catches updates; it does **not** catch deletes. A deleted
  entity stays in the dictionary until the `MAX 600` lifetime
  expires. All generated dimension joins use `dictGetOrNull` (not
  `dictGet`) so that missing-key rows surface as `NULL` rather than
  silently rendering as empty string — see
  [Warehouse SCOPE W13](../Warehouse/SCOPE.md#w13--dimension-joins-must-surface-missing-entries-explicitly).
  The lag is surfaced at every read envelope via the
  `dimension_freshness` block — see
  [W11](../Warehouse/SCOPE.md#w11--dimension-staleness-is-bounded-documented-and-surfaced).
- **No FKs on ClickHouse history.** `samples.entity_id` is a string,
  not an FK. Orphaned samples (entity deleted in Postgres, samples
  still in ClickHouse) are possible and will persist until the TTL
  expires (up to 2 years for cold samples). The operator primer in
  [Warehouse SCOPE](../Warehouse/SCOPE.md) includes an orphan audit
  query using `dictGetOrNull` to count affected rows and targeted
  `ALTER TABLE … DELETE` for early cleanup. The W13 `dictGetOrNull`
  rule ensures orphaned rows surface as `NULL` in dashboards rather
  than silently rendering with empty dimension values.
- **No real UPDATE/DELETE on history.** Lightweight DELETE works for
  emergency cleanup; routine row mutation is not supported. The
  history is append-only by design.

### What this ADR does NOT do

- **Does not move OLTP tables to ClickHouse.** `flows`, `agents`,
  `runs`, agent sessions, skill approvals, users, sessions, plus
  `entities`, `entity_refs`, `marts`, `tag_definitions` all stay
  in Postgres. ADR-001 still holds for OLTP.
- **Does not adopt ClickHouse Cloud or any ClickHouse Inc.
  proprietary feature.** Self-hosted OSS only. If a future feature
  needs ClickPipes or SharedMergeTree, it gets its own ADR.
- **Does not adopt streaming ingest (Kafka, Pulsar, CDC).** Writes
  come from the flow engine over HTTP. Streaming sources are a
  future option, not part of this decision.
- **Does not commit the warehouse to ClickHouse at the trait layer.**
  The traits in `starter-warehouse-spi` (TBD in Warehouse SCOPE)
  stay backend-agnostic. Today there is one impl in
  `starter-store-clickhouse`. A future Parquet/DuckDB read-side, or
  a return to single-DB Postgres for a smaller deployment, remains
  possible without changing consumers.
- **Does not guarantee point-in-time-consistent recovery across
  Postgres and ClickHouse.** Each store has its own backup pipeline
  and its own RPO. A restore that puts Postgres at T1.5 with
  ClickHouse at T2 (or vice versa) leaves orphaned history rows
  with no matching `entities` row, or `entities` rows with no
  history. The W13 `dictGetOrNull` rule guarantees orphan rows
  render as `[unknown entity]` rather than corrupt joins, and the
  orphan audit query in the operator primer is the recovery tool.
  Operators who need cross-store PITR should script coordinated
  snapshots (PG `pg_basebackup` + CH `BACKUP TABLE` at the same
  wall-clock instant) and accept the residual seconds-to-minutes
  skew as the recovery boundary.

## Rationale

Two pressures decided the call:

1. **Cold-storage cost.** Bounded retention cost over years is
   non-negotiable. Timescale's tiered-storage answer sits in the TSL
   licensing tier; ADR-002 self-imposed the Apache-2 lane to keep
   licensing trivial, which structurally cut off the cold-tier
   answer. ClickHouse delivers tiered S3 storage in the Apache-2
   OSS build with no asterisks.
2. **Two databases is the price; the price is acceptable.** Running
   two databases costs real ops effort. We weighed that against (a)
   accepting Timescale + TSL licensing, (b) accepting Timescale +
   homegrown cold-tier scripts, or (c) accepting the two-DB split
   with each database doing what it is genuinely best at. Option (c)
   is the lowest long-term-risk path. The seam is narrow and
   one-directional, the consumer surface is unchanged
   (`TagQuery`-only at the read edge), and the OLTP side is
   *literally* what we already had before the warehouse existed.

ClickHouse is mature (Cloudflare, Uber, GitHub-style scale users in
production for years), Apache-2.0 across the OSS build, and runs as
a single static binary in its simplest deployment. The `JSON` type
went GA in 25.3 (early 2025) but we deliberately choose `Map(String,
String)` for tags — it has been production-stable for years and the
`bloom_filter` skip index path is well-trodden.

## References

- [ADR-001 — `flow-agent` is Postgres-only (OLTP)](./ADR-001-flow-agent-postgres-only.md)
- [ADR-002 — Warehouse on TimescaleDB (RETIRED)](./ADR-002-timescaledb-same-db.RETIRED.md)
- [Tags SCOPE](../Tags/SCOPE.md) — two compilation targets (PG + CH)
- [Warehouse SCOPE](../Warehouse/SCOPE.md) — the L1/L2/L3 model on
  ClickHouse, marts catalog in Postgres, dimension dictionaries
- ClickHouse docs:
  [JSON / Map / skip indexes](https://clickhouse.com/docs/optimize/skipping-indexes),
  [incremental materialized views](https://clickhouse.com/docs/materialized-view/incremental-materialized-view),
  [async inserts](https://clickhouse.com/blog/asynchronous-data-inserts-in-clickhouse),
  [TTL to S3](https://clickhouse.com/docs/observability/managing-data),
  [Dictionary with PostgreSQL source](https://clickhouse.com/docs/dictionary),
  [LICENSE — Apache 2.0](https://github.com/ClickHouse/ClickHouse/blob/master/LICENSE)
