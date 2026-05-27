# docs/sessions/

> 🪦 **Historical-record notice (2026-05-27).** Any session note in
> this directory that mentions ClickHouse, `ChClient`, `ChWriter`,
> `rubix.clickhouse.*` verbs, or `starter-store-clickhouse` reflects
> the **pre-warehouse-engine-swap** state of the codebase. The
> warehouse engine was replaced with **TimescaleDB** (loaded into the
> existing Postgres instance) and surfaces were renamed to
> `warehouse-*` / `rubix.warehouse.*` / `starter-store-warehouse`.
> Authoritative refs:
> [ADR-004](../../../DOCS/storage/ADR-004-timescaledb-warehouse.md)
> and the
> [warehouse-engine-swap proposal](../proposal/warehouse-engine-swap.md).
> Do not "fix" historical notes by rewriting — they document past
> design intent.

**Tier:** Throwaway working notes. Lifetime: days.

Per [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md):

- One file per session, named `YYYY-MM-DD-<topic>.md`.
- Exploratory. Ugly is fine. Delete when promoted.
- **Never** referenced from source code.

When a session note settles into a real implementation, **promote**
it to a `docs/design/<area>/README.md` per HOW-TO-CODE §0a and
delete (or archive) the session note.
