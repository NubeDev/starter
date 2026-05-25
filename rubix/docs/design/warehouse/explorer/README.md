# Warehouse Explorer (design)

> **Placeholder.** This page exists so source code in
> [`crates/starter-warehouse/src/explorer/`](../../../../../crates/starter-warehouse/src/explorer/)
> can satisfy [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) (code may
> reference `docs/design/`, never `docs/scope/`).
>
> The present-tense design lands in PR 4 of the
> [ClickHouse Explorer plan](../../scope/clickhouse-explorer.md). Until
> then the scope doc is the source of truth — read it for the *why*
> and the PR sequencing. This file just gives the code a stable link
> target.

The explorer is a read-only HTTP sub-router inside `starter-warehouse`
that exposes ClickHouse `system.*` queries — table list, per-table
schema and size, columns, ERD, autocomplete — under
`/api/warehouse/ch/*`, gated by the `warehouse.read` permission. It
reuses the existing `ChClient` from `starter-store-clickhouse` so no
second connection or auth path is introduced.

Writes never go through the explorer. Anything that mutates state
flows through the existing `rubix.clickhouse.*` verbs so the
snapshot-before-write + undo + changelog contract is preserved.
