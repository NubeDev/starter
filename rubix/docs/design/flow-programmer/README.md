# flow-programmer

Present-tense design note for the rubix flow-programmer goal.
Covers the four verbs bound to the `com.rubix.flow-programmer`
flow — two write verbs (`rubix.flow_ops.deploy`,
`rubix.flow_ops.duplicate`) and two read verbs
(`rubix.flow_ops.lint`, `rubix.flow_ops.list`) — plus
`rubix.undo.last`, the snapshot shape both write verbs'
`Reversible` impl reads and writes, the cross-instance
`NOTIFY rubix_flows_definitions` mechanism that propagates a
deploy to every rubix-agent process in the cluster, and the
deploy contract every caller honours. End-to-end coverage lives
in `rubix-agent/tests/goal_3_flow_programmer_test.rs`: a single
scenario seeds `com.rubix.scheduled-system-check` as a live
revision, dispatches `flow_ops.duplicate` through the
`UndoDispatcher` to copy it to a `-copy` target, asserts a new
revision row landed and `flow_ops.list` surfaces both, dispatches
`rubix.undo.last`, then asserts the new revision is superseded
and `flow_ops.list` reverts to surfacing the source alone.

## Verb surface

| Verb                          | Op       | MessageKey(s)                                              | Reversible? |
| ----------------------------- | -------- | ---------------------------------------------------------- | ----------- |
| `rubix.flow_ops.deploy`       | Create   | `rubix.flow.deployed`, `rubix.flow.deploy.invalid`         | yes         |
| `rubix.flow_ops.duplicate`    | Create   | `rubix.flow.duplicated`                                    | yes (first-revision under target) |
| `rubix.flow_ops.lint`         | Read     | `rubix.flow.linted`, `rubix.flow.lint.found_errors`        | no          |
| `rubix.flow_ops.list`         | Read     | `rubix.flow.listed`                                        | no          |
| `rubix.undo.last`             | n/a      | (delegates to `starter-undo`)                              | n/a         |

Each verb implements `starter_spi::tool::Tool` for invocation and
(for the writers) `rubix_tools::undo::dispatch::ReversibleTool`
for the `change_for` adapter that emits an optional
`starter_undo::ChangeDraft` from the `(input, output)` pair on the
success path. The dispatcher (`UndoDispatcher`) forwards the draft
to `starter_undo::record_if_reversible`, which records the change
through the workspace `ChangeRecorder` so a later
`rubix.undo.last` walks it back via `FlowDefReversible`.

## Backing store

All four verbs talk to a single per-goal trait —
`rubix_tools::flow_ops::store::FlowDefStore`. The trait is a thin
seam so the production binary swaps a PG-backed impl in without
touching the verb files. Today the only impl is the in-memory
`InMemoryFlowDefStore` used by the unit tests and the agent-loop
integration test; the production impl wiring is the
`flows_definitions` PG table from Phase D.1, queried through
`rubix_store_postgres`.

The trait shape mirrors the snapshot-before-write contract:
`insert_revision` returns `(inserted_row, prior_revision_id)` so
the verb stamps the `Change` envelope without a second probe;
`mark_superseded` / `clear_superseded` are the two primitives the
`FlowDefReversible` impl uses to walk a deploy or duplicate
backwards.

## `flows_definitions` table and the deploy contract

`rubix_store_postgres` migration `<NNNN>_flows_definitions.sql`
defines the dimension table the verbs land rows into:

| Column          | Type        | Notes                                          |
| --------------- | ----------- | ---------------------------------------------- |
| `id`            | TEXT PK     | ULID (one per row)                             |
| `tenant_id`     | TEXT        | tenancy scope                                  |
| `flow_id`       | TEXT        | reverse-DNS flow id (rows for one flow share)  |
| `revision_id`   | TEXT        | UUID per row                                   |
| `body_yaml`     | TEXT        | raw YAML body, persisted verbatim              |
| `created_at`    | TIMESTAMPTZ | insertion timestamp                            |
| `created_by`    | TEXT        | actor subject from the `Actor::User` envelope  |
| `superseded_at` | TIMESTAMPTZ | NULL when the revision is live                 |

A `UNIQUE` constraint on `(tenant_id, flow_id, revision_id)`
catches accidental duplicate inserts.

The deploy contract every caller honours:

1. The verb parses `body_yaml` through `rubix_flows::parse_yaml`.
   Parse failure short-circuits with `rubix.flow.deploy.invalid`
   and writes no row.
2. The verb cross-checks the body's `id:` field against the
   request's `flow_id`. Mismatch short-circuits with
   `rubix.flow.deploy.invalid` (the operator passed a body that
   does not belong under the requested id).
3. `FlowDefStore::insert_revision` writes a fresh row with
   `superseded_at = NULL` and marks the previously-live row (if
   any) for the same `flow_id` superseded `at now()`. Atomic
   under a single PG transaction in the production impl.
4. The verb stamps a `Change` with `Op::Create`, `after =
   FlowDefChange { flow_id, revision_id, prior_revision_id }`,
   `before = None`. The dispatcher records it through
   `ChangeRecorder`.

`duplicate` is the same contract with two extra invariants: the
source flow must have a live revision (else `NotFound`), the
target flow must **not** have a live revision (else `Conflict`),
and the body's `id:` field is rewritten to the target before the
insert.

## Cross-instance NOTIFY mechanism

The migration installs a trigger on `flows_definitions` that
fires `pg_notify('rubix_flows_definitions', <payload>)` on every
`INSERT` and `UPDATE`. The payload JSON carries
`{ op, id, tenant_id, flow_id, revision_id, superseded_at }`.

Every rubix-agent process spawns one `PgListener` at boot
(`rubix_agent::boot::flow_notify`) on the
`rubix_flows_definitions` channel. On every notify the listener
filters to non-superseded events, re-reads the row from the table
(payloads have an 8000-byte limit, so the body is fetched
authoritatively from the table not the payload), parses the
`body_yaml` through `rubix_flows::parse_yaml` +
`rubix_flows::convert`, and calls a `ReloadFn` hook with the
`(FlowId, FlowRevisionId, FlowBody)` triple. The hook is wired by
`boot::mcp::register` to `FlowRegistry::reload` so the
in-process registry picks up the new revision without a
redeploy. Every connected MCP client sees the new revision on the
next `tools/list`.

The same channel carries supersede events: when
`FlowDefReversible::apply_inverse` clears the new revision's
`superseded_at` (rolling a deploy back), the row's `UPDATE` fires
the trigger, every listener re-reads the row, and the in-process
registry reloads to the prior live body. `rubix.undo.last`
therefore propagates cluster-wide via the same path a fresh
deploy uses — no out-of-band reload RPC.

When `dsn` is `None` (laptop boot without Postgres) the listener
short-circuits with `Ok(None)` and the in-process registry stays
the only source of truth.

## Snapshot shape

`starter_spi::changelog::Change` carries an `Op` plus optional
`before` / `after` JSON payloads. The flow-programmer goal owns
one resource kind:

### `kind = "flow_definition"`

`Op::Create` (both `deploy` and `duplicate`):

- `before`: `None` (the new revision is by definition fresh).
- `after`: `FlowDefChange { flow_id, revision_id,
  prior_revision_id }`.
- `prior_revision_id` is `Some(rev)` when the verb superseded an
  earlier live revision under the same `flow_id` (the deploy
  path against an existing flow), and `None` when the new row is
  the first live revision under that `flow_id` (the duplicate
  path, and the first deploy of a fresh flow).
- Inverse: `FlowDefStore::mark_superseded(revision_id, now)`
  retires the new revision. If `prior_revision_id` is
  `Some(rev)`, `FlowDefStore::clear_superseded(rev)` restores it
  as the live row.
- Forward replay: symmetric — `mark_superseded(prior, now)` then
  `clear_superseded(revision_id)`.

The `clone_with` path on `FlowDefReversible` is intentionally
unwired: `rubix.flow_ops.duplicate` owns the clone, and the
changelog-level `clone_with` would duplicate the routing.

## Lint contract

`rubix.flow_ops.lint` is a structural read. The verb parses the
body through `rubix_flows::parse_yaml` then runs
`rubix_flows::convert` so semantic errors (empty `nodes:`, bad
`id:`, missing kinds at registry time) surface alongside YAML
syntax errors. Each error becomes a `LintDiagnostic { message,
line, column }`; `line` and `column` populate when the underlying
`serde_yaml::Error::location()` carries them. A clean body emits
`rubix.flow.linted` with an empty `errors[]`; a failing body
emits `rubix.flow.lint.found_errors` with `count = errors.len()`.

Skill prompts gate `rubix.flow_ops.deploy` behind a successful
lint (SKILL.md §"How to work" item 1).

## Localisation

The six MessageKeys land alongside the verb files (Phase D.2):

- `rubix.flow.deployed` — `{flow_id}`, `{at}`
- `rubix.flow.deploy.invalid` — `{detail}`
- `rubix.flow.duplicated` — `{source}`, `{target}`, `{at}`
- `rubix.flow.linted` — (no params)
- `rubix.flow.lint.found_errors` — `{count}`
- `rubix.flow.listed` — `{count}`

Entries land in both `rubix-spi/catalogues/en.json` and
`rubix-spi/catalogues/es.json` in the same commit that fills the
verbs (workspace rule R5).

## Idempotence and undo

`deploy` is **not** idempotent: every call writes a fresh
revision row even when the body matches the live revision
byte-for-byte. Callers (and the flow-programmer skill prompt)
de-dup at the prose layer when that matters. The undo path walks
exactly one revision back per `rubix.undo.last` call.

`duplicate` refuses to overwrite a live target (`Conflict`), so
the no-op path is "do nothing"; on the success path it always
produces a draft and the inverse retires the freshly-created
revision.

`lint` and `list` write no state and produce no `ChangeDraft`.
