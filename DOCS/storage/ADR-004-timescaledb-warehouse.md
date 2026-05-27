# ADR-004 — Warehouse runs on TimescaleDB inside the same Postgres

- **Status:** Accepted
- **Date:** 2026-05-27
- **Scope:** The `starter-warehouse` capability and any consumer
  (starting with the rubix agent) that needs a tag-driven analytics
  layer with dashboard-grade read latency and years-of-history
  retention.
- **Supersedes:**
  [ADR-003 — ClickHouse warehouse, Postgres OLTP](./ADR-003-clickhouse-warehouse.RETIRED.md).
- **Implemented by:**
  [`rubix/docs/proposal/warehouse-engine-swap.md`](../../rubix/docs/proposal/warehouse-engine-swap.md)
  (the authoritative design — cite that doc, not this ADR, for
  hypertable / continuous-aggregate / retention / chunk-sizing
  detail).
- **Related (unchanged):** ADR-001 (`flow-agent` is Postgres-only
  for OLTP).

## Decision

The warehouse runs on **TimescaleDB**, loaded as an extension into the
existing Postgres instance. There is **one database**: dimensions
(`entities`, `entity_refs`, `tag_definitions`, `marts` catalog, …)
live in regular Postgres tables; history (`raw_events`, `samples`,
`events`, `documents`) lives in TimescaleDB **hypertables** in the same
database; rollup marts are **continuous aggregates** with a refresh
policy; retention is `add_retention_policy` per hypertable.

ClickHouse is gone. The `starter-store-clickhouse` crate, the
`docker-compose.clickhouse.yml` service, the `entities_dict`
materialized dictionary, the `async_insert=1 / wait_for_async_insert=1`
discipline, and the `rubix.clickhouse.*` verb namespace were deleted in
stage 3 of the warehouse-engine-swap.

## Surface names

The public surface is **engine-neutral** — `warehouse`, not
`clickhouse`:

| Old (retired)                                | Current                                      |
| -------------------------------------------- | -------------------------------------------- |
| `starter-store-clickhouse` crate             | `starter-store-warehouse` crate              |
| `ChWriter` trait                             | `WarehouseWriter` trait                      |
| `Ch{Rule,Mart,Retention}Reversible`          | `Warehouse{Rule,Mart,Retention}Reversible`   |
| `rubix.clickhouse.*` verbs                   | `rubix.warehouse.*` verbs                    |
| `clickhouse-ruler` skill / flow YAML         | `warehouse-ruler` skill / flow YAML          |
| `rubix_tools::clickhouse::mart` module       | `rubix_tools::warehouse::mart` module        |

The `DdlDialect` trait kept in `starter-store-warehouse` keeps the
door open for another engine later; only the `TimescaleDb` impl ships.

## Why

The full rationale, including continuous-aggregate constraints,
chunk-time-interval choices, `tenant_id`-in-`GROUP BY` for RLS, and
the trade-offs against ClickHouse, lives in the proposal:
[`rubix/docs/proposal/warehouse-engine-swap.md`](../../rubix/docs/proposal/warehouse-engine-swap.md).
This ADR records the **decision**; the proposal records the **design**.

## Doc-hygiene note for AI assistants

Many historical documents under
[`rubix/docs/sessions/`](../../rubix/docs/sessions/),
[`.codeless/jobs/`](../../.codeless/jobs/), and older design docs
still reference ClickHouse, `ChClient`, `rubix.clickhouse.*`,
`system_disk_history` on a MergeTree, etc. Those references describe
the pre-2026-05-27 state and are kept as historical record only.
**For current design, cite this ADR or the proposal above.** When in
doubt, treat any `clickhouse` reference in the repo as historical
unless it appears in this ADR or in `warehouse-engine-swap.md`.
