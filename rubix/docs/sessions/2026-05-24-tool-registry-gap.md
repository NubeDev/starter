# 2026-05-24 — Tool registry gap: SDK calls verbs the agent never registered

> **Tier:** session note. Lifetime: days. Per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md), source code must
> never reference this file.

## What happened

A smoke of the rubix SPA against a freshly-booted agent surfaced a
wall of 404s on `POST /api/v1/tools/<tool_id>`. The
[frontend-surfaces session note](2026-05-24-frontend-surfaces.md)
had explicitly claimed Phase C.1 "found every insights + clickhouse
list/CRUD endpoint already present on `rubix-agent/src/routes/`" —
which was wrong. The grep was against the typed-client TypeScript
package (`packages/rubix-client-ts/src/endpoints/`), not the
agent's registered tools.

Concretely, the agent's `build_tool_registry()` registered only 5
of the ~26 tool ids the frontend SDK calls. The full audit:

| Verb id                                | Rust type in `rubix-tools` | Status after this branch |
| -------------------------------------- | -------------------------- | ----------------------- |
| `rubix.flow_ops.list`                  | `FlowListTool`             | wired (InMemory + seeded) |
| `rubix.flow_ops.lint`                  | `FlowLintTool`             | wired                   |
| `rubix.flow_ops.deploy`                | `FlowDeployTool`           | wired (InMemory)        |
| `rubix.flow_ops.duplicate`             | `FlowDuplicateTool`        | wired (InMemory)        |
| `rubix.user.list`                      | `UserListTool`             | wired (InMemory)        |
| `rubix.user.create`                    | `UserCreateTool`           | wired (InMemory)        |
| `rubix.user.disable`                   | `UserDisableTool`          | wired (InMemory)        |
| `rubix.tenant.list`                    | `TenantListTool`           | wired (InMemory)        |
| `rubix.team.create`                    | `TeamCreateTool`           | wired (InMemory)        |
| `rubix.team.assign`                    | `TeamAssignTool`           | wired (InMemory)        |
| `rubix.clickhouse.rule.list`           | `ClickhouseRuleListTool`   | **NEW** + wired         |
| `rubix.clickhouse.rule.write`          | `ClickhouseRuleWriteTool`  | wired (was unregistered)|
| `rubix.clickhouse.mart.list`           | `ClickhouseMartListTool`   | **NEW** + wired         |
| `rubix.clickhouse.mart.create`         | `ClickhouseMartCreateTool` | wired (was unregistered)|
| `rubix.clickhouse.mart.drop`           | `ClickhouseMartDropTool`   | **NEW** + wired         |
| `rubix.clickhouse.tables.list`         | `ClickhouseTablesListTool` | **NEW** + wired         |
| `rubix.clickhouse.retention.set`       | `ClickhouseRetentionSetTool` | wired (was unregistered)|
| `rubix.insights.rule.list`             | `InsightsRuleListTool`     | **NEW** + wired         |
| `rubix.insights.rule.create`           | `InsightsRuleCreateTool`   | **NEW** + wired         |
| `rubix.insights.rule.enable`           | `InsightsRuleEnableTool`   | **NEW** + wired         |
| `rubix.insights.rule.disable`          | `InsightsRuleDisableTool`  | **NEW** + wired         |
| `rubix.analytics.report`               | `AnalyticsReportTool`      | deferred — needs `BlobStore` |
| `rubix.analytics.query`                | `AnalyticsQueryTool`       | deferred — no SDK call site |
| `rubix.undo.last`                      | `UndoLastTool`             | deferred — needs `UndoService` + `ActorSource` |

## What landed in this PR

### Commit `dd698ab` — flow_ops + user + tenant + team

[`rubix/crates/rubix-agent/src/registry.rs`](../../crates/rubix-agent/src/registry.rs)
now registers the ten verbs in the upper block, backed by the
existing `InMemory*` stores. `InMemoryFlowDefStore` is **seeded
from `rubix_flows::bundled()`** at boot, so
`rubix.flow_ops.list` returns the six canonical flows on a fresh
agent.

### Follow-up commit — clickhouse + insights

Eleven additional verbs registered:

- **Three already implemented but never wired**:
  `clickhouse.rule.write`, `clickhouse.mart.create`,
  `clickhouse.retention.set` — direct wire-up against a shared
  `Arc<dyn ChWriter>` (the existing `InMemoryChWriter`).

- **Four new ClickHouse verbs**: `rule.list`, `mart.list`,
  `mart.drop`, `tables.list`. Backed by three new `ChWriter`
  trait methods (`list_rules`, `list_marts`, `list_tables`) with
  default empty-vec impls so any pre-existing fake keeps
  compiling. `InMemoryChWriter` returns the union of marts and
  TTL-tracked tables for `list_tables` with a constant engine
  name; the CH-backed swap returns real `system.tables` rows.
  `mart.drop` walks `ChWriter::restore_mart` with an empty
  snapshot (the same code path the undo dispatcher uses).

- **Four new insights verbs**: `rule.list`, `rule.create`,
  `rule.enable`, `rule.disable`. New `InsightsRuleStore` trait
  with an `InMemoryInsightsStore` impl. `create` is an idempotent
  upsert; toggle verbs return a `rubix.insights.rule.not_found`
  diagnostic (not an error) when the id is unknown so the SPA
  can render a friendly toast.

Live smoke against the running agent (cookie + CSRF, post-login):

```
$ for v in rubix.clickhouse.{rule.list,rule.write,mart.list,mart.create,mart.drop,tables.list,retention.set} \
           rubix.insights.rule.{list,create,enable,disable}; do
    curl -s -o /dev/null -w "%{http_code}  $v\n" \
      -b jar -H "x-csrf-token: $CSRF" -H 'content-type: application/json' \
      -X POST "http://127.0.0.1:8088/api/v1/tools/$v" -d "$body"
  done
200  rubix.clickhouse.rule.list
200  rubix.clickhouse.rule.write
200  rubix.clickhouse.mart.list
200  rubix.clickhouse.mart.create
200  rubix.clickhouse.mart.drop
200  rubix.clickhouse.tables.list
200  rubix.clickhouse.retention.set
200  rubix.insights.rule.list
200  rubix.insights.rule.create
200  rubix.insights.rule.enable
200  rubix.insights.rule.disable
```

Boot log confirms `tools=26 mcp_tools=6` (was `tools=5` before
this branch).

A boot-time `warn!` on `rubix.registry` makes the in-memory
backing state visible in the boot log so an operator does not
mistake the empty lists for a working production surface.

Sixteen new unit tests pin the wired verb ids and per-verb
behaviour so silent re-deregistration cannot happen in the future
(8 in the agent's `registry::tests`, 6 in
`rubix-tools::clickhouse::*::tests` for the four new verbs, 6 in
`rubix-tools::insights::*::tests` for the new family).

## Follow-ups (NOT in this PR)

### F1 — PG-backed `UserAdminStore` adapter

`rubix.user.list` currently shows zero users even on an agent that
booted with a working PG and a `rubix-admin bootstrap-user` row.
The operator cannot manage users through the admin UI until this
adapter lands.

Cheapest correct shape: a thin
`PgUserAdminStore` in `rubix/crates/rubix-store-postgres/` that
wraps `starter_auth_users::store::user_store::UserStore` and maps
its `User` row → `rubix_tools::user::store::UserRow`. The trait
shape is the contract (see `user/store.rs` module docs); construction
is a single `Arc` substitution in `registry.rs`.

### F2 — PG-backed `FlowDefStore`

Today the seeded `InMemoryFlowDefStore` survives a `flow_ops.list`
but **a `flow_ops.deploy` followed by an agent restart loses the
new revision** (the seed wipes the in-memory state on next boot).
The flow-programmer integration test
(`goal_3_flow_programmer_test.rs`) explicitly documents the swap
as "a one-line change in the agent boot wiring".

Implementation: a `flow_definitions` table in
`rubix-store-postgres/migrations/` plus a `PgFlowDefStore` impl. The
LISTEN/NOTIFY channel
(`boot::spawn_flow_notify`) is already wired and would start
emitting reload signals as soon as the PG store is in place.

### F3 — PG-backed `TenantStore` / `TeamAdminStore`

Same shape as F1/F2. Lower priority since the admin UI's
tenant/team surfaces are not on the critical path for the
rubix-agent smoke flow.

### F4 — CH-backed `ChWriter` impl (replaces `InMemoryChWriter`)

The seven `rubix.clickhouse.*` verbs are now wired but land DDL
against an in-process `HashMap`. Production needs a `ChWriter`
impl over `starter_store_clickhouse::ChClient`:

- `show_create_*` → `SHOW CREATE TABLE <name>` (catch
  `UNKNOWN_TABLE` as `Ok(None)`).
- `apply_*_ddl` → execute the DDL, then `SHOW CREATE TABLE` to
  produce the post-state snapshot.
- `list_rules` / `list_marts` → `SELECT name, create_table_query
  FROM system.tables WHERE database = currentDatabase()` with a
  family-discriminator predicate (engine starts with
  `MaterializedView` for rules; `MergeTree`-family for marts).
- `list_tables` → `SELECT name, engine, total_rows, ttl FROM
  system.tables`.
- `apply_retention` → `ALTER TABLE ... MODIFY TTL ...`.

The trait is already in the right shape; only the impl needs to
land.

### F5 — PG-backed `InsightsRuleStore` + `rubix.undo.last` + analytics

- `InsightsRuleStore`: tiny PG table `insights_rules(rule_id PK,
  name, enabled, body_yaml, updated_at)` — straight `UPSERT`,
  `UPDATE`, `SELECT`. After this lands the four insights verbs
  survive restart.
- `rubix.undo.last`: requires constructing an `UndoService` (needs
  the changelog SQLite/PG impl already wired in
  `boot::build_auth`) and an `ActorSource` that reads the
  authenticated `Principal` from request extensions. The
  `goal_3_flow_programmer_test.rs` shows the dispatcher seam.
- `rubix.analytics.report`: needs a `BlobStore` impl wired (the
  agent does not yet construct one).
- `rubix.analytics.query`: needs the live `ChClient` threaded
  through (already constructed for `DiskTool`).

## Hard rule reminder

R2 (upstream-first) applies. The PG store impls land in
`rubix-store-postgres/`, not in the rubix-agent binary's
`registry.rs`. The CH-backed `ChWriter` impl lands in
`starter-store-clickhouse/`. The binary only wires
`Arc::new(PgFooStore::new(pool))` /
`Arc::new(ChClientWriter::new(client))`.

