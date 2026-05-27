# warehouse-rules

> **Current state (2026-05-27):** this document was written against
> the ClickHouse-era surface and has drifted from the
> TimescaleDB-backed implementation that landed in PR #44
> (warehouse-engine-swap) and the rebuild that followed. Specifically:
>
> - The engine is **TimescaleDB**, not ClickHouse. The
>   `WarehouseWriter` trait / `InMemoryWarehouseWriter` /
>   `ChClient` referenced below were deleted in stage 3 of the
>   engine swap. Verbs now talk directly to
>   `starter_store_warehouse::WarehouseClient` (a `sqlx::PgPool`
>   wrapper) via the helpers in
>   `starter_store_warehouse::{cagg, retention}`.
> - The verb set is **four writes**, not three. `mart.drop` was
>   added to give `mart.create` a forward inverse that doesn't
>   depend on the undo path.
> - **No `Reversible` impls exist in source** for any of the four
>   writes. `WarehouseRuleReversible` / `WarehouseMartReversible` /
>   `WarehouseRetentionReversible` are aspirational types named in
>   doc comments but never written. The write verbs return
>   `prior_ddl` / `prior_days` in their response payloads so
>   callers can snapshot externally; nothing reaches
>   `undo_snapshots`.
> - **The production undo runtime is not wired at all.** See
>   [`../undo/README.md`](../undo/README.md) for the full picture —
>   `UndoDispatcher`, `ReversibleRegistry`, and `UndoLastTool` are
>   never constructed in `main.rs`, so the "Reversible? yes"
>   column below describes the intended shape, not the runtime.
> - **Rule vs. mart is naming-convention only.** Both are
>   TimescaleDB continuous aggregates; `rule.list` filters caggs
>   whose `view_name` matches `_rule$` or `^rule_`, `mart.list`
>   filters the complement. A registration table that records the
>   kind explicitly (alongside other per-cagg metadata like the
>   creating actor and the original DDL) is the natural follow-up
>   when undo / governance lands.
>
> The remainder of this document describes the intended snapshot
> shapes that the deferred Reversible impls will use. Treat it as
> a design target, not as a description of running code.

Present-tense design note for the rubix warehouse-ruler goal.
Covers the four write verbs bound to the `com.rubix.warehouse-ruler`
flow — `rubix.warehouse.rule.write`, `rubix.warehouse.mart.create`,
`rubix.warehouse.mart.drop`, `rubix.warehouse.retention.set` —
plus `rubix.undo.last`, and the snapshot shape every write verb's
`Reversible` impl will read and write once the runtime is wired.

## Verb surface

| Verb                                | Op       | MessageKey(s)                                                              | Reversible? |
| ----------------------------------- | -------- | -------------------------------------------------------------------------- | ----------- |
| `rubix.warehouse.rule.write`       | Update   | `rubix.warehouse.rule.written`, `rubix.warehouse.rule.invalid`           | yes         |
| `rubix.warehouse.mart.create`      | Create   | `rubix.warehouse.mart.created`, `rubix.warehouse.mart.already_exists`    | yes (first-create only — the idempotent re-call records no Change) |
| `rubix.warehouse.retention.set`    | Update   | `rubix.warehouse.retention.set`, `rubix.warehouse.retention.unchanged`   | yes (first-change only — the no-op re-call records no Change) |
| `rubix.undo.last`                   | n/a      | (delegates to `starter-undo`)                                              | n/a         |

Each verb implements `starter_spi::tool::Tool` for invocation and
`rubix_tools::undo::dispatch::ReversibleTool` for the `change_for`
adapter that emits an optional `starter_undo::ChangeDraft` from the
`(input, output)` pair on the success path. The dispatcher
(`UndoDispatcher`) forwards the draft to
`starter_undo::record_if_reversible`, which records the change
through the workspace `ChangeRecorder` so a later `rubix.undo.last`
walks it back via the per-kind `Reversible` impl
(`WarehouseRuleReversible`, `WarehouseMartReversible`, `WarehouseRetentionReversible`).

## Backing store

All three verbs talk to a single per-goal trait —
`rubix_tools::warehouse::store::WarehouseWriter`. The trait is a thin
seam so the production binary can swap a CH-backed impl in without
touching the verb files. Today the only impl is the in-memory
`InMemoryWarehouseWriter` used by the unit tests and the agent-loop
integration tests; the production impl wiring lands in a follow-up
stage that consumes `starter-store-warehouse::ChClient`.

The trait shape mirrors the snapshot-before-write contract: every
mutator returns `(prior_snapshot, new_snapshot)` so the verb stamps
the `Change` envelope without a second probe. Prior state comes
from `SHOW CREATE TABLE <name>` (rule and mart kinds) or a TTL
query against `system.tables` (retention kind).

## Snapshot shape

`starter_spi::changelog::Change` carries an `Op` plus optional
`before` / `after` JSON payloads. Each rubix resource kind owns the
JSON layout for those payloads; the matching `Reversible` impl
interprets them.

### `kind = "clickhouse_rule"`

`Op::Update`:

- `before`: `WarehouseRuleSnapshot { rule_name, ddl: Option<String> }`
  carrying the `SHOW CREATE TABLE` body captured immediately before
  the write, or `ddl = None` when the rule did not exist.
- `after`: `WarehouseRuleSnapshot { rule_name, ddl: Some(new_ddl) }`.
- Inverse: when `before.ddl = Some(body)`, replay the body
  verbatim. When `before.ddl = None`, issue `DROP TABLE IF EXISTS
  <rule_name>` — the rule did not exist before, so undo restores
  the empty state.

### `kind = "clickhouse_mart"`

`Op::Create`:

- `before`: `WarehouseMartSnapshot { mart_name, ddl: None }` for the fresh
  create path. The verb refuses to record a Change at all for the
  idempotent re-create path (`was_already_present = true`).
- `after`: `WarehouseMartSnapshot { mart_name, ddl: Some(new_ddl) }`.
- Inverse: `DROP TABLE IF EXISTS <mart_name>`. See the data-loss
  caveat below.

### `kind = "clickhouse_retention"`

`Op::Update`:

- `before`: `WarehouseRetentionSnapshot { table_name, days: Option<u32> }`
  carrying the TTL the table had at snapshot time; `days = None`
  means the table had no TTL clause.
- `after`: `WarehouseRetentionSnapshot { table_name, days: Option<u32> }`
  carrying the new TTL; `days = None` means the verb removed the
  TTL clause (`days = 0` in the request).
- Inverse: re-issue `ALTER TABLE <name> MODIFY TTL …` against the
  `before` value, or `ALTER TABLE <name> REMOVE TTL` when
  `before.days = None`.

## `mart.create` undo data-loss caveat

`rubix.warehouse.mart.create` is the only verb in this goal whose
undo path is **not** state-preserving. The snapshot captured before
the write is `WarehouseMartSnapshot { ddl: None }` (the mart did not
exist), so the only inverse op consistent with that snapshot is
`DROP TABLE IF EXISTS`. The schema is restored to its pre-create
state, but every row ingested between the create and the undo is
discarded with the table.

Operators driving the clickhouse-ruler skill see this surfaced in
two places:

1. The skill prompt (`SKILL.md` §"How to work" item 4) tells the
   model to warn before suggesting `rubix.undo.last` against a
   freshly-created mart.
2. The verb DTO (`WarehouseMartCreateResponse`) documents the
   caveat on the `prior_ddl` field, and the descriptor's
   `when_not_to_use` notes that DROP goes through the undo path,
   not a direct verb.

The other two verbs — `rule.write` and `retention.set` — capture a
non-empty prior (the previous DDL body or the previous TTL) and
roll cleanly back without data loss.

## Localisation

Six MessageKeys land alongside the verb files (Phase C.1):

- `rubix.warehouse.rule.written` — `{rule}`, `{at}`
- `rubix.warehouse.rule.invalid` — `{preview}`
- `rubix.warehouse.mart.created` — `{mart}`, `{at}`
- `rubix.warehouse.mart.already_exists` — `{mart}`, `{at}`
- `rubix.warehouse.retention.set` — `{table}`, `{days}`, `{at}`
- `rubix.warehouse.retention.unchanged` — `{table}`, `{days}`, `{at}`

Entries land in both `rubix-spi/catalogues/en.json` and
`rubix-spi/catalogues/es.json` in the same commit that fills the
verbs (workspace rule R5).

## Idempotence and undo

`mart.create` and `retention.set` are idempotent — re-calling them
against an already-present mart, or with a TTL value that matches
the current TTL, returns the matching `already_exists` /
`unchanged` outcome code (with a boolean flag set on the typed
response) and produces **no** `ChangeDraft`. The dispatcher's
`change_for` returns `None` on the no-op path, so undo can never
silently unwind a state the caller did not actually flip.

`rule.write` always produces a draft on success; its inverse is
either the prior DDL body (when one was captured) or a
`DROP TABLE IF EXISTS` against the rule name (when the rule did
not exist before).
