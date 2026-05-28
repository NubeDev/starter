---
date: 2026-05-28
---

# 2026-05-28 — Tenant lifecycle: `rubix.tenant.create` + `rubix.tenant.delete`

Natural follow-up to the §3.3 close earlier today. Closes the
**cascade-on-tenant-delete** decision that was deferred from the
`rubix.user.tenant.assign` session ("recorded so it gets debated
rather than implicitly made when a delete verb appears"). The
verb appeared this session; the decision is made.

Picked this over the three "next-session candidates" in the
proposal because all three were blocked:

- §3.2 node-level undo — still gated on Phase B `flow_nodes`.
- Operator surface for `changelog_kind_policy` — the concurrent
  admin session touched `rubix-agent/src/admin/*.rs` and
  `routes/admin/*.rs` ~50 min before this session started; still
  in flight. Deferred to avoid edit-buffer collisions.
- Warehouse-write Reversibles — deferred until operator demand.

Tenant lifecycle has zero overlap with the concurrent session,
sits cleanly inside the user-admin scope, and closes a real
recorded decision.

## Cascade-on-delete decision: **refuse-if-users-assigned**

Three alternatives weighed; one chosen:

| Option | Behaviour | Verdict |
| --- | --- | --- |
| Cascade-unassign | Silently flip every assigned user to `tenant_id = None` on delete. | **Rejected.** Fans out N audit rows the operator didn't ask for; crosses actor boundaries (the deleter may not own the affected users); lets an operator delete-then-recreate to forcibly unassign users from a tenant they don't technically own. |
| Block at the FK | Same effective behaviour as refuse, but with an opaque DB-side error. | **Rejected** for operator-experience reasons; the refusal message must carry the assignment count and the tenant name so the operator can act on it. |
| Refuse with structured diagnostic | Return `Error::Conflict` whose payload is the `rubix.tenant.has_users` diagnostic carrying `{tenant, name, count}`. | **Chosen.** |

The operator workflow is now:

1. `rubix.tenant.delete({tenant_id: "t-acme"})`
2. → `Error::Conflict { rubix.tenant.has_users { tenant: "t-acme", name: "Acme", count: 3 } }`
3. Operator runs `rubix.user.list`, filters by `tenant_id`, and
   issues `rubix.user.tenant.assign({user_id: ..., tenant_id:
   null})` three times.
4. `rubix.tenant.delete({tenant_id: "t-acme"})` succeeds.

Symmetrically — undo of step 4 only restores the tenant row.
Re-attaching the users requires running undo three more times
(once per `rubix.user.tenant.assign`). This is the right shape:
each mutation lives in its own audit row, owned by the actor who
performed it.

## `rubix.tenant.create`

Non-idempotent. Duplicate `tenant_id` OR duplicate `name`
returns `Error::Conflict`. Tenants are identity boundaries;
making create silent-idempotent would let two operators each
think they "own" a tenant they share. Better to surface the
collision.

- `tenant_id` optional — generated as `t-<uuid>` when absent
  (mirrors `u-<uuid>` posture).
- `locale` optional — defaults to `"en"` when absent.
- `name` required, trimmed, non-empty.
- Empty `Some("")` for `tenant_id` / `locale` rejected as
  `Error::Invalid` (`Some("")` is almost always a wire-shaped
  bug; same posture as `tenant.assign`).

Snapshot shape: `Op::Create`, `after` = the full new `TenantRow`,
`before = None`. Standard create-shaped Reversible — undo
deletes the row.

## `rubix.tenant.delete`

Refuse-on-assigned-users; otherwise hard-delete. The verb takes
**two** stores — `Arc<dyn TenantStore>` + `Arc<dyn UserAdminStore>`
— same posture as today's `rubix.user.tenant.assign`. The
user-store consultation is the FK check; if any
`UserRow.tenant_id == target`, the verb refuses.

Snapshot shape: `Op::Delete`, `before` = the full prior
`TenantRow`, `after = None`. `change_for` reconstructs the
`before` from the response payload — the row no longer exists
in the store post-delete, so every identity-bearing field
(`tenant_id`, `name`, `locale`) rides on the response.

## `TenantReversible`

New `Reversible` impl for the `tenant` kind, snapshot shape,
full-row payload. Registered in `registry.rs` alongside
`UserReversible`, `TeamReversible`, `DashboardReversible`,
`FlowDefReversible`.

`apply_inverse` deliberately bypasses the user-presence check:

> Per-actor redo-stack semantics (proposal §3.4) keep this safe
> in the normal case: a single actor's undo chain walks back in
> reverse mutation order, so the user assignments are unwound
> before the tenant create is undone. If a different actor
> inserts user assignments between the create and the undo, the
> undo may delete a tenant that has users assigned — that's a
> cross-actor concurrency boundary, the same shape as every
> other Reversible in the codebase (see `DashboardReversible`
> for the precedent).

This matches the snapshot-faithful replay contract — the
Reversible doesn't second-guess the recorded operation, the verb
does.

## Audit-floor pin

New seed migration
[`changelog_policy/0002_tenant_audit_floor.sql`](../../../crates/rubix-store-postgres/migrations/changelog_policy/0002_tenant_audit_floor.sql):

```sql
INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES
    ('tenant', NULL)
ON CONFLICT (resource_kind) DO NOTHING;
```

Tenant lifecycle is as security-relevant as role / disable on
users. Same `max_age_days = NULL` posture as `user` and `team`
from the `0001_audit_floor_seed.sql` migration. The audit floor
for the `tenant` kind is now recorded in SQL, not tribal memory.

## Store changes

`TenantStore` grew from a read-only single-method trait to a
full CRUD surface:

- `list()` — unchanged.
- `get(tenant_id)` — added in the morning `tenant.assign`
  session; left in place.
- `create(row)` — new. Returns `Error::Conflict` on duplicate id
  OR duplicate name.
- `put(row)` — new. Bypasses uniqueness checks (snapshot
  restoration must succeed even mid-transient-conflict).
- `delete(tenant_id)` — new. Returns `Error::NotFound` on
  unknown id. Does NOT enforce the user-presence check — that
  is verb-level because the user store is in a sibling module.

`InMemoryTenantStore` switched its backing from `Vec<TenantRow>`
to `HashMap<String, TenantRow>` for O(1) id lookups. The
`seeded(rows)` constructor still works (used by the registry
boot path to install the bundled `System` tenant). The legacy
`insert` helper is retained for tests but documented as
test-only.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/tenant/create.rs` — DTO + descriptor.
- `rubix/crates/rubix-spi/src/dto/tenant/delete.rs` — DTO + descriptor.
- `rubix/crates/rubix-tools/src/tenant/create.rs` — verb + 7 unit tests.
- `rubix/crates/rubix-tools/src/tenant/delete.rs` — verb + 7 unit tests.
- `rubix/crates/rubix-store-postgres/migrations/changelog_policy/0002_tenant_audit_floor.sql`
- `rubix/docs/sessions/undo/2026-05-28-tenant-lifecycle.md` (this doc).

### Modified

- `rubix/crates/rubix-spi/src/dto/tenant/mod.rs` — `pub mod {create,delete};`
- `rubix/crates/rubix-spi/catalogues/en.json` — three keys
  (`rubix.tenant.created`, `.deleted`, `.has_users`).
- `rubix/crates/rubix-spi/catalogues/es.json` — three keys.
- `rubix/crates/rubix-tools/src/tenant/mod.rs` — `pub mod {create,delete};`
- `rubix/crates/rubix-tools/src/tenant/store.rs` — full rewrite:
  `HashMap` backing, full CRUD trait surface, new
  `TenantReversible` impl, `TENANT_KIND` constant.
- `rubix/crates/rubix-agent/src/registry.rs` — `TenantReversible`
  in `ReversibleRegistry`, `TenantCreateTool` + `TenantDeleteTool`
  wired through `wrap_rev` alongside the existing
  `TenantListTool`.
- `rubix/docs/design/undo/README.md` — tenant-lifecycle subsection
  added, pointing at this doc and at the cascade decision.

## Validation

- `cargo test -p rubix-tools --lib` → **218 passed** (was 200 at
  session start; +14 from new verbs and +4 store tests). All
  morning's user / tenant_assign tests still green — the store
  rewrite is wire-additive at the trait level.
- `cargo test -p rubix-agent --test goal_2_user_admin_test --test
  undo_dispatch_test --test admin_registry_test --test
  admin_openapi_projection_test --test changelog_sweep_test` →
  unchanged green. The registry test and the OpenAPI projection
  both pick up the new verbs without further wiring.
- `cargo build --workspace` — clean.
- Concurrent admin session files (`admin/`, `routes/admin/`,
  `boot/auth.rs`, `tail_listen.rs`) — **not touched**, verified
  via `git status --short`.

## What's next

- **Operator surface for `changelog_kind_policy` /
  `undo_kind_policy`** — once the concurrent admin session
  stabilises (file mtimes go cold for a working day). The verbs
  to add are `rubix.audit.policy.get` (read all rows) and
  `rubix.audit.policy.set({kind, max_age_days})` (operator
  override, ON CONFLICT UPDATE). Both need to route through
  whatever the admin session settles on for admin-only tools.
- **`rubix.tenant.update`** — rename / re-locale. Out of scope
  this session because rename has a snapshot-fidelity question
  (does the diagnostic carry the prior name?) that deserves
  its own slice. Pattern: copy `rubix.user.role.set`.
- **PG-backed `TenantStore`** — the in-memory store survives
  this session; production still loses tenants on restart. The
  `starter-auth-users::PgTenantStore` already exists for the
  auth-side store; either wire that as the rubix-tools store too,
  or write a `PgRubixTenantStore` in `rubix-store-postgres`.
  Recorded in store.rs's module doc.
- **§3.2 / warehouse Reversibles** — still blocked.
