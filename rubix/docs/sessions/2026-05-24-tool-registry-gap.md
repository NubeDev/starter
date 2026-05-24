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

Concretely:

| Verb id                            | Rust type in `rubix-tools` | Registered in `build_tool_registry` |
| ---------------------------------- | -------------------------- | ----------------------------------- |
| `rubix.flow_ops.list`              | `FlowListTool`             | **no** (now yes)                    |
| `rubix.flow_ops.lint`              | `FlowLintTool`             | **no** (now yes)                    |
| `rubix.flow_ops.deploy`            | `FlowDeployTool`           | **no** (now yes)                    |
| `rubix.flow_ops.duplicate`         | `FlowDuplicateTool`        | **no** (now yes)                    |
| `rubix.user.list`                  | `UserListTool`             | **no** (now yes)                    |
| `rubix.user.create`                | `UserCreateTool`           | **no** (now yes)                    |
| `rubix.user.disable`               | `UserDisableTool`          | **no** (now yes)                    |
| `rubix.tenant.list`                | `TenantListTool`           | **no** (now yes)                    |
| `rubix.team.create`                | `TeamCreateTool`           | **no** (now yes)                    |
| `rubix.team.assign`                | `TeamAssignTool`           | **no** (now yes)                    |
| `rubix.clickhouse.mart.list`       | —                          | **no Rust impl**                    |
| `rubix.clickhouse.mart.drop`       | —                          | **no Rust impl**                    |
| `rubix.clickhouse.rule.list`       | —                          | **no Rust impl**                    |
| `rubix.clickhouse.tables.list`     | —                          | **no Rust impl**                    |
| `rubix.analytics.report`           | `AnalyticsReportTool`      | **no** (deferred — needs `BlobStore`) |
| `rubix.analytics.query`            | `AnalyticsQueryTool`       | **no** (deferred — call site unknown) |
| `rubix.undo.last`                  | `UndoLastTool`             | **no** (deferred — needs `UndoService` + `ActorSource`) |

## What landed in this PR

[`rubix/crates/rubix-agent/src/registry.rs`](../../crates/rubix-agent/src/registry.rs)
now registers the ten verbs in the upper block, backed by the
existing `InMemory*` stores. `InMemoryFlowDefStore` is **seeded
from `rubix_flows::bundled()`** at boot, so
`rubix.flow_ops.list` returns the six canonical flows on a fresh
agent. The user / tenant / team stores start empty (mutations land
in-memory; lost on restart).

```
$ curl -sb jar -X POST http://127.0.0.1:8088/api/v1/tools/rubix.flow_ops.list \
    -H "x-csrf-token: $CSRF" -d '{}'
{"count":6,"flows":[{"flow_id":"com.rubix.clickhouse-ruler", ...}]}

$ curl -sb jar -X POST http://127.0.0.1:8088/api/v1/tools/rubix.user.list \
    -H "x-csrf-token: $CSRF" -d '{}'
{"count":0,"summary":{"code":"rubix.user.listed", ...},"users":[]}

$ curl -sb jar -X POST http://127.0.0.1:8088/api/v1/tools/rubix.tenant.list \
    -H "x-csrf-token: $CSRF" -d '{}'
{"count":0,"summary":{"code":"rubix.tenant.listed", ...},"tenants":[]}
```

A boot-time `warn!` on `rubix.registry` makes the in-memory backing
state visible in the boot log so an operator does not mistake the
empty lists for a working production surface.

Six new registry unit tests pin the wired verb ids
(`registry_contains_flow_ops_quartet`,
`registry_contains_user_admin_verbs`,
`registry_contains_tenant_and_team_verbs`,
`flow_store_is_seeded_from_bundled_flows`) so silent
re-deregistration cannot happen in the future.

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
tenant/team surfaces are not on the critical path for the rubix-agent
smoke flow.

### F4 — ClickHouse list/admin verbs

`rubix.clickhouse.mart.list`, `rubix.clickhouse.mart.drop`,
`rubix.clickhouse.rule.list`, `rubix.clickhouse.tables.list` have no
Rust types yet — the typed TypeScript client and the React hooks
were written ahead of the backend. The DTOs need to land in
`rubix-spi/src/dto/clickhouse/` first, then the tools in
`rubix-tools/src/clickhouse/`. The read verbs can be implemented as
thin wrappers over the ClickHouse `system.tables` query the existing
`ChClient` already supports.

The frontend hooks
([`rubix/packages/rubix-client-react/src/hooks/clickhouse.ts`](../../packages/rubix-client-react/src/hooks/clickhouse.ts))
already carry an inline comment acknowledging this gap ("the
backing tool ids do not yet exist in `@nube/rubix-client-ts` — the
agent-side endpoints are still being landed — see the stage 9
BLOCKED handover").

### F5 — `rubix.undo.last`, `rubix.analytics.{report,query}`

Lower priority — none of these surfaced in the operator's 404
report. Wire when:

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
`registry.rs`. The binary only wires `Arc::new(PgFooStore::new(pool))`.
