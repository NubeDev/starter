# ADR-002 — Warehouse runs on TimescaleDB in the *same* Postgres database

> 🪦 **RETIRED 2026-05-23** — superseded by
> [ADR-003 — ClickHouse warehouse, Postgres OLTP](./ADR-003-clickhouse-warehouse.md).
> The single-database constraint this ADR pinned was dropped in
> exchange for ClickHouse's cold-S3 tiering (Apache-2 OSS) and
> per-insert materialized-view freshness. The rest of this document
> is kept verbatim as the historical record of why we considered
> Timescale and what tipped the call. **Do not cite for current
> design — cite ADR-003.**

- **Status:** Retired (superseded by ADR-003)
- **Date:** 2026-05-23
- **Scope:** The forthcoming `starter-warehouse` capability and any
  consumer (starting with [`examples/flow-agent`](../../examples/flow-agent))
  that needs a tag-driven analytics layer.
- **Supersedes:** Nothing. Extends
  [ADR-001](./ADR-001-flow-agent-postgres-only.md) — that ADR pinned
  *Postgres-only* for `flow-agent`; this one pins *TimescaleDB
  extension, same database* for the warehouse layer.

## Context

The product needs a place to land **arbitrary incoming data** (driver
telemetry, webhook payloads, flow outputs), **tag it**, **roll it up
for dashboards**, and let an **AI agent compose pages by tag query**.
Read latency on dashboards is the headline KPI — pages must open in
<200 ms regardless of how many years of history sit underneath.

Three architectures were considered (see the conversation that drove
this ADR):

1. **Vanilla Postgres only.** Marts as plain tables or materialised
   views, refreshed by flow jobs. Simple, but timeseries scale and
   incremental rollup are hand-rolled.
2. **DuckDB only / DuckDB primary.** Excellent for analytics, wrong
   for OLTP: single-writer, no real concurrent transactions, no FKs
   worth relying on. The product is multi-user with concurrent
   driver ingest — DuckDB-only is structurally wrong.
3. **TimescaleDB extension in the existing Postgres database.**
   Timeseries hypertables, **continuous aggregates** (incremental
   self-refreshing marts), columnar compression, retention
   policies — all in vanilla SQL, all in the database the product
   already uses.

The deciding factor: **continuous aggregates are exactly the L3
mart shape we'd otherwise build by hand**, with correctness
properties (incremental, no double-count on retry, no missed
chunks) that a flow-scheduled rollup job cannot match cheaply.

## Decision

**The warehouse capability runs on TimescaleDB, installed as an
extension into the same Postgres database that already holds flows,
agents, runs, agent sessions, skill approvals, and every other
OLTP-shaped table.** No second database. No second connection pool.
No second backup story.

Concretely:

- `starter-store-postgres` gains one new cargo feature, `warehouse`,
  default-off.
- A new migration namespace `crates/starter-store-postgres/migrations/warehouse/`
  follows the existing namespacing convention (per-feature subdir,
  `_sqlx_migrations_warehouse` version table). Mirrors `flow/`,
  `agent_sessions/`, `skills/`.
- `migrations/warehouse/0001_init.sql` runs
  `CREATE EXTENSION IF NOT EXISTS timescaledb;` then creates the
  warehouse tables (entities, samples hypertable, marts catalog,
  raw landing zone).
- All warehouse SQL is **standard Postgres + Timescale Apache-2
  features**. No TSL/commercial-source-available functions are used.
- The `PgPool` is shared across every store module. One pool, one
  set of credentials, one transaction boundary.

## Consequences

### Positive

- **One database, full stop.** Same backup, same restore, same
  observability, same connection pool, same migration runner, same
  ops on-call playbook.
- **Cross-table joins and transactions stay possible.** A flow run
  can write to `runs`, `samples`, and `marts` in one transaction.
- **Continuous aggregates as built-in marts.** Mart refresh
  becomes a `add_continuous_aggregate_policy(...)` call, not a
  flow-scheduled rollup job. Less code in the engine, more
  correctness in the database.
- **Hypertable scale.** Timescale auto-partitions `samples` by
  time. A single Postgres holds 10s–100s of millions of rows per
  table without index pain.
- **Compression + retention.** `samples` chunks older than 7 days
  compress 10×+ (column-store internally); chunks older than the
  retention policy drop automatically. SD-card-killing append rates
  on a Pi remain a separate problem
  (see [ADR-001](./ADR-001-flow-agent-postgres-only.md)) — but on a real
  disk, the warehouse cost is bounded by the retention policy, not
  by uptime.
- **Migration story is unchanged.** Same `embed_migrations!` pattern
  per namespace that the workspace already runs.

### Negative

- **Deployment image must include Timescale.** The official
  `postgres:16` image does *not* ship the extension. Deployments
  switch to `timescale/timescaledb:latest-pg16` (or a self-built
  image that adds the `timescaledb` shared library). This is a
  one-line change in compose files and Helm charts — but it is a
  real change.
- **Testcontainers must use the Timescale image.** The existing
  `starter-store-postgres::testing` helper gains a
  `with_timescale()` variant that pins the Timescale image. Tests
  that exercise warehouse migrations require this; tests that only
  touch non-warehouse tables can stay on vanilla Postgres.
- **One more extension to know about.** Operators need to understand
  hypertables enough to read EXPLAIN plans and run
  `chunk_compression_stats('samples')`. We document the shortlist
  in [Warehouse SCOPE](../Warehouse/SCOPE.md) under "Operator
  primer."
- **Apache-2 lane only.** We do not use TSL-licensed Timescale
  features (advanced compression tunables, a few specialised
  functions). This is a self-imposed constraint to keep the
  licensing story trivial. If a future feature truly needs a TSL
  function, it gets its own ADR.

### What this ADR does NOT do

- **Does not move existing OLTP tables onto hypertables.** `flows`,
  `agents`, `runs`, agent sessions, skill approvals all stay as
  ordinary Postgres tables. Hypertables are an opt-in per table
  via `create_hypertable(...)` — only `samples` (and any future
  high-rate timeseries table) gets that treatment.
- **Does not adopt DuckDB.** DuckDB remains a future read-side
  escape hatch *if and when* a quantified problem emerges
  (e.g. ad-hoc analytics scans regularly exceeding 1 s on
  realistic data volumes). The warehouse design keeps that door
  open by treating marts as **declarative catalog entries** —
  the backend that materialises a mart can change without the
  dashboard or `mart.read` consumer noticing.
- **Does not adopt pgvector.** If the AI agent or document search
  ever needs semantic similarity over entity descriptions or
  curated documents, `pgvector` is the obvious extension (also a
  Postgres extension; same-DB story stays intact). Out of scope
  for this ADR; would warrant a one-paragraph ADR-003 when needed.
- **Does not commit `starter-warehouse` to PG-only at the trait
  layer.** The store traits in `starter-spi` (or in a new
  `starter-warehouse-spi`, TBD in Warehouse SCOPE) stay
  backend-agnostic. Today there is one implementation in
  `starter-store-postgres`. SQLite is not getting a warehouse
  implementation (no hypertables, no continuous aggregates — the
  shape doesn't fit), and that is fine: warehouse-using deployments
  pick Postgres, the same way `flow-agent` does per ADR-001.

## Rationale

**The single-database constraint is non-negotiable for this
product.** Multi-database architectures cost an order of magnitude
more in ops, observability, and developer cognitive load than they
appear to on day one. We do not have the team size to absorb that
cost, and the workload does not require it.

Timescale is the unique option that gives us **columnar-grade read
performance for the analytics workload, OLTP semantics for the rest,
and a single Postgres** to operate. DuckDB beats it on raw scan
speed for huge cold history — but the moment you need DuckDB *and*
concurrent ingest *and* user transactions, you need two systems,
and we are explicitly refusing that.

We accept Timescale's deployment-image cost in exchange for keeping
the system at one database for the foreseeable future.

## References

- [ADR-001 — `flow-agent` is Postgres-only](./ADR-001-flow-agent-postgres-only.md)
- [Tags SCOPE](../Tags/SCOPE.md) — the tag language and query grammar
  consumed by the warehouse
- [Warehouse SCOPE](../Warehouse/SCOPE.md) — the L1/L2/L3 model,
  node kinds, marts catalog, AI authoring contract
- [`crates/starter-store-postgres`](../../crates/starter-store-postgres) —
  where the `warehouse` feature and migrations will land
- TimescaleDB docs: hypertables, continuous aggregates, compression,
  retention policies (Apache-2 features only)
