# Starter changes — Phase 4 gates

> 🪦 **Historical — pre-2026-05-27.** ClickHouse references below
> describe the original Phase 4 plan. The warehouse engine was swapped
> to TimescaleDB; the warehouse surface is now engine-neutral
> (`starter-store-warehouse`, `rubix.warehouse.*` verbs,
> `warehouse-ruler` flow). See
> [ADR-004](../../../../DOCS/storage/ADR-004-timescaledb-warehouse.md)
> and the [warehouse-engine-swap proposal](../../proposal/warehouse-engine-swap.md).

Warehouse + cron (Goals 4 and 5) gates — written when the warehouse
ran on ClickHouse; kept verbatim as historical context.

See [README.md](./README.md) for the index and per-item format.

## `cron-schedule` node kind in `starter-flow-nodes`

- **Crate:** `starter-flow-nodes`
- **Blocks rubix phase:** 4 (Goal 5 cron triggering)
- **Why upstream:** every flow consumer wants cron triggers.
- **Status:** planned (verify whether `starter-flow` Service
  surface already covers this before opening a new node kind)
- **Notes:** if Service surface covers it, the upstream item moves
  to "document how the Service surface implements cron" rather
  than a new node kind.

## `starter-tool-clickhouse` — rule.write / mart.create / retention.set

- **Crate:** `starter-tool-clickhouse` (new)
- **Blocks rubix phase:** 4 (Goal 4 ClickHouse)
- **Why upstream:** any starter consumer with ClickHouse benefits.
- **Status:** planned
- **Notes:** rubix consumes; same fallback rules as Phase 3 tools.

## `clickhouse-query` node kind in `starter-flow-nodes`

- **Crate:** `starter-flow-nodes` (optional)
- **Blocks rubix phase:** 4 (Goal 4 ClickHouse)
- **Why upstream:** any starter consumer with ClickHouse benefits.
- **Status:** conditional — only file if rubix has a flow YAML
  that calls ClickHouse directly. If all ClickHouse access stays
  inside `rubix-tools` Rust code, no upstream item needed.
