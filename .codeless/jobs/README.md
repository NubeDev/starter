# .codeless/jobs/

**Historical record of completed and in-flight migration jobs.**

> 🪦 **Notice for AI assistants (2026-05-27).** Any job record in this
> directory that mentions ClickHouse, `ChClient`, `ChWriter`,
> `rubix.clickhouse.*` verbs, `starter-store-clickhouse`, or
> `clickhouse-ruler` reflects the **pre-warehouse-engine-swap** state
> of the codebase.
>
> The warehouse engine was replaced with **TimescaleDB** (loaded as an
> extension into the existing Postgres instance, same DB as OLTP).
> Surfaces were renamed to `warehouse-*` / `rubix.warehouse.*` /
> `starter-store-warehouse` / `warehouse-ruler`.
>
> Authoritative references:
> - [DOCS/storage/ADR-004-timescaledb-warehouse.md](../../DOCS/storage/ADR-004-timescaledb-warehouse.md)
> - [rubix/docs/proposal/warehouse-engine-swap.md](../../rubix/docs/proposal/warehouse-engine-swap.md)
>
> **Do not "fix" historical job records by rewriting** — they
> document past design intent and the decision trail that led to the
> current architecture. Treat ClickHouse references here as evidence,
> not code-to-update.
