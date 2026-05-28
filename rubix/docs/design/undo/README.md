# UNDO

> **Current state (2026-05-28):** the production runtime is wired.
> `rubix-agent`'s `main.rs` constructs a `registry::UndoSubstrate`
> from the live PG pool (`PgChangeRecorder` + `PgChangeLog` +
> `PgUndoCursor`), passes it to `build_tool_registry`, which (a)
> wraps every `ReversibleTool` (dashboards, users, teams, flow_ops,
> warehouse — 11 verbs total) in `UndoDispatcher`, (b) appends
> `rubix.undo.last` and `rubix.undo.redo` as callable verbs, and (c)
> applies the `starter_undo_cursors` migration alongside the
> changelog migration so the per-actor redo stack is durable from
> first boot. The REST `/api/v1/tools/*` handler installs the
> caller's `Actor` into a `tokio` task-local so `UndoDispatcher`
> sees the right actor without threading it through every
> `Tool::invoke` signature. The substrate is `Option<…>`: laptops
> without `mcp_url` (no PG) skip the wiring and fall back to
> changelog-only behaviour.


Every reversible write the rubix backend dispatches lands in
`starter_changes`, and any actor can roll back their last group with
`rubix.undo.last`. The wiring has three pieces:

1. **`starter_undo::ReversibleRegistry`** — one
   `starter_spi::changelog::Reversible` impl per resource kind. Built
   once at agent boot and shared as an `Arc` with every dispatcher.
2. **`starter_undo::dispatch::record_if_reversible`** — the helper
   the dispatch wrapper calls after a successful domain mutation.
   Looks up the resource kind in the registry; if found, opens a
   `ChangeRecorder::transaction` and writes one row with the
   `(before, after, op, resource, actor)` the tool supplied.
   Returns the assigned `GroupId`. Unregistered kinds short-circuit
   to `Ok(None)` — read-only verbs and tools that have no Reversible
   counterpart never touch the recorder.
3. **`rubix_tools::undo::dispatch::UndoDispatcher`** — the
   `Tool`-shaped wrapper used at the agent boundary. It calls the
   inner `Tool::invoke`, hands the `(input, output)` pair to the
   tool's `ReversibleTool::change_for` adapter to build a
   `ChangeDraft`, then forwards to `record_if_reversible`. Tools
   that have no Reversible adapter implement `Tool` only and
   bypass the wrapper.

The verb that closes the loop is **`rubix.undo.last`**
(`rubix_tools::undo::last::UndoLastTool`). It pulls the calling
`Actor` from an `ActorSource` (the agent loop's request context) and
calls `starter_undo::undo_last(service, actor, scope)`, which today
delegates to `UndoService::undo` and walks the actor's most recent
group. The `scope` parameter is reserved for a per-resource filter
the goal-2/3/4 work introduces; the verb already accepts it so the
client contract does not change when the filter activates.

## Adding a new reversible resource

1. Implement `starter_spi::changelog::Reversible` for the resource
   and register the impl with `ReversibleRegistry::insert` at boot.
2. Implement `ReversibleTool::change_for` on the tool that mutates
   it; return `Some(ChangeDraft)` describing the before/after
   snapshot pair.
3. Wrap the tool with `UndoDispatcher::new(inner, registry,
   recorder, actor)` in the agent's tool registry.

Nothing else changes — the dispatcher, helper, and `rubix.undo.last`
verb are kind-agnostic.

## Snapshot vs patch policy

See the rustdoc on `starter_spi::changelog::Reversible` for the full
decision matrix; the short version: snapshot is the default, patch is
the escape hatch when the row is large or the delta is small. Each
in-tree impl tags its choice (see the policy section of its module
doc).

## Dashboard metadata fold (proposal §3.1 decision)

`rubix.dashboard.update` and `rubix.dashboard.patch` write the page
body **and** carry title/tags. There is no separate
`rubix.dashboard.definition` verb today. The Reversible impl folds
metadata into the page snapshot: `DashboardSnapshot` carries
`title`, `tags`, and `body_json` together, so undo of a rename
restores all three atomically. The chokepoint
`DashboardStore::insert_revision_with_prior` returns the superseded
row's metadata in `prior_title` / `prior_tags`, which `change_for`
threads into the `before` snapshot.

This is a pragmatic fold rather than the architecturally cleaner
"separate `rubix.dashboard.definition` kind" the proposal mentions
as an alternative. Revisit when either:

1. A metadata-only verb appears (today only `update` can touch
   title/tags, and it always writes the body too), or
2. A "rename history" view wants to query metadata edits in
   isolation from body edits.

Until then, the folded snapshot is correct and round-trips byte-exact.

## Outstanding gaps

- **Warehouse write verbs have no Reversible impls.** The four
  TimescaleDB-backed writes — `rubix.warehouse.rule.write`,
  `rubix.warehouse.mart.create`, `rubix.warehouse.mart.drop`,
  `rubix.warehouse.retention.set` — return `prior_ddl` /
  `prior_days` in their response payloads (so callers can snapshot
  externally) but no `impl Reversible` exists for them and the
  values are not persisted to `undo_snapshots`. Snapshot shapes are
  documented in
  [`../warehouse-rules/README.md`](../warehouse-rules/README.md);
  the impls themselves are deferred until an operator demand
  surfaces — until then, warehouse writes are advertised as
  one-way in the verb descriptors.
- **User role / prefs Reversible.** Proposal §3.3 explicitly
  defers this until a separate audit-log proposal exists. Don't
  extend undo retention to substitute for audit. The audit-log
  proposal now exists at
  [`../../proposal/audit-log.md`](../../proposal/audit-log.md) —
  recommendation is to add an `audit_only_*` floor on
  `undo_kind_policy` so security-relevant kinds persist past undo
  retention, then ship the §3.3 extension on top.

## Tests

- **`starter_undo::dispatch::tests`** — unit-level round-trip
  through a fake `Reversible` and an in-memory recorder.
- **`rubix_agent` integration test `undo_dispatch_test.rs`** —
  three tests: (1) registers a fake tool + Reversible, dispatches
  through the live `SqliteChangeRecorder`, asserts the recorded row
  drives the inverse path; (2) pins the proposal §3.4
  redo-clear-on-mutation invariant; (3) guards that unregistered
  kinds don't touch the cursor.
- **`starter_undo::tests::cursor_postgres`** — five
  docker-backed tests covering the `PgUndoCursor` epoch CAS:
  round-trip, isolation, persistence, concurrent CAS, and
  agent-run-id keying. Run with
  `cargo test -p starter-undo --features postgres --test cursor_postgres -- --ignored`.
- **`rubix_agent::tests::undo_redo_e2e_test`** — single
  docker-backed end-to-end pinning the create → update → undo →
  redo → clear-on-mutation → cross-process-replay sequence through
  `UndoDispatcher::with_cursor` + `PgUndoCursor` + the
  `rubix.undo.{last,redo}` verbs. Run with
  `cargo test -p rubix-agent --test undo_redo_e2e_test -- --ignored`.

Both `--ignored` suites are wired into the CI `undo-postgres` job
([`.github/workflows/ci.yml`](../../../../.github/workflows/ci.yml))
so a regression in the cursor or dispatch wiring fails the PR check
instead of waiting on a developer remembering `-- --ignored` locally.

