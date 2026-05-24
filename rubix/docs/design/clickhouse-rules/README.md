# clickhouse-rules

Present-tense design note for the rubix clickhouse-ruler goal.
Covers the three write verbs bound to the `com.rubix.clickhouse-ruler`
flow — `rubix.clickhouse.rule.write`, `rubix.clickhouse.mart.create`,
`rubix.clickhouse.retention.set` — plus `rubix.undo.last`, and the
snapshot shape every write verb's `Reversible` impl reads and writes.
End-to-end coverage lives in
`rubix-agent/tests/goal_4_clickhouse_ruler_test.rs`: the single
scenario dispatches `retention.set` through the `UndoDispatcher`,
asserts the `ALTER TABLE … MODIFY TTL` ran, asserts the changelog
carries a snapshot row with the prior/new TTL, then dispatches
`rubix.undo.last` and asserts the prior TTL is restored.

## Verb surface

| Verb                                | Op       | MessageKey(s)                                                              | Reversible? |
| ----------------------------------- | -------- | -------------------------------------------------------------------------- | ----------- |
| `rubix.clickhouse.rule.write`       | Update   | `rubix.clickhouse.rule.written`, `rubix.clickhouse.rule.invalid`           | yes         |
| `rubix.clickhouse.mart.create`      | Create   | `rubix.clickhouse.mart.created`, `rubix.clickhouse.mart.already_exists`    | yes (first-create only — the idempotent re-call records no Change) |
| `rubix.clickhouse.retention.set`    | Update   | `rubix.clickhouse.retention.set`, `rubix.clickhouse.retention.unchanged`   | yes (first-change only — the no-op re-call records no Change) |
| `rubix.undo.last`                   | n/a      | (delegates to `starter-undo`)                                              | n/a         |

Each verb implements `starter_spi::tool::Tool` for invocation and
`rubix_tools::undo::dispatch::ReversibleTool` for the `change_for`
adapter that emits an optional `starter_undo::ChangeDraft` from the
`(input, output)` pair on the success path. The dispatcher
(`UndoDispatcher`) forwards the draft to
`starter_undo::record_if_reversible`, which records the change
through the workspace `ChangeRecorder` so a later `rubix.undo.last`
walks it back via the per-kind `Reversible` impl
(`ChRuleReversible`, `ChMartReversible`, `ChRetentionReversible`).

## Backing store

All three verbs talk to a single per-goal trait —
`rubix_tools::clickhouse::store::ChWriter`. The trait is a thin
seam so the production binary can swap a CH-backed impl in without
touching the verb files. Today the only impl is the in-memory
`InMemoryChWriter` used by the unit tests and the agent-loop
integration tests; the production impl wiring lands in a follow-up
stage that consumes `starter-store-clickhouse::ChClient`.

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

- `before`: `ChRuleSnapshot { rule_name, ddl: Option<String> }`
  carrying the `SHOW CREATE TABLE` body captured immediately before
  the write, or `ddl = None` when the rule did not exist.
- `after`: `ChRuleSnapshot { rule_name, ddl: Some(new_ddl) }`.
- Inverse: when `before.ddl = Some(body)`, replay the body
  verbatim. When `before.ddl = None`, issue `DROP TABLE IF EXISTS
  <rule_name>` — the rule did not exist before, so undo restores
  the empty state.

### `kind = "clickhouse_mart"`

`Op::Create`:

- `before`: `ChMartSnapshot { mart_name, ddl: None }` for the fresh
  create path. The verb refuses to record a Change at all for the
  idempotent re-create path (`was_already_present = true`).
- `after`: `ChMartSnapshot { mart_name, ddl: Some(new_ddl) }`.
- Inverse: `DROP TABLE IF EXISTS <mart_name>`. See the data-loss
  caveat below.

### `kind = "clickhouse_retention"`

`Op::Update`:

- `before`: `ChRetentionSnapshot { table_name, days: Option<u32> }`
  carrying the TTL the table had at snapshot time; `days = None`
  means the table had no TTL clause.
- `after`: `ChRetentionSnapshot { table_name, days: Option<u32> }`
  carrying the new TTL; `days = None` means the verb removed the
  TTL clause (`days = 0` in the request).
- Inverse: re-issue `ALTER TABLE <name> MODIFY TTL …` against the
  `before` value, or `ALTER TABLE <name> REMOVE TTL` when
  `before.days = None`.

## `mart.create` undo data-loss caveat

`rubix.clickhouse.mart.create` is the only verb in this goal whose
undo path is **not** state-preserving. The snapshot captured before
the write is `ChMartSnapshot { ddl: None }` (the mart did not
exist), so the only inverse op consistent with that snapshot is
`DROP TABLE IF EXISTS`. The schema is restored to its pre-create
state, but every row ingested between the create and the undo is
discarded with the table.

Operators driving the clickhouse-ruler skill see this surfaced in
two places:

1. The skill prompt (`SKILL.md` §"How to work" item 4) tells the
   model to warn before suggesting `rubix.undo.last` against a
   freshly-created mart.
2. The verb DTO (`ClickhouseMartCreateResponse`) documents the
   caveat on the `prior_ddl` field, and the descriptor's
   `when_not_to_use` notes that DROP goes through the undo path,
   not a direct verb.

The other two verbs — `rule.write` and `retention.set` — capture a
non-empty prior (the previous DDL body or the previous TTL) and
roll cleanly back without data loss.

## Localisation

Six MessageKeys land alongside the verb files (Phase C.1):

- `rubix.clickhouse.rule.written` — `{rule}`, `{at}`
- `rubix.clickhouse.rule.invalid` — `{preview}`
- `rubix.clickhouse.mart.created` — `{mart}`, `{at}`
- `rubix.clickhouse.mart.already_exists` — `{mart}`, `{at}`
- `rubix.clickhouse.retention.set` — `{table}`, `{days}`, `{at}`
- `rubix.clickhouse.retention.unchanged` — `{table}`, `{days}`, `{at}`

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
