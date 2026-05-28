---
date: 2026-05-28
---

# 2026-05-28 — Undo / redo: §3.3 `rubix.user.tenant.assign` verb — **§3.3 closed**

Closes proposal §3.3. Ships the third and last of the three
planned user verbs (`role.set` / `prefs.set` / `tenant.assign`).
Bigger than the prior two because it is a **model change** — the
user row gained a new field — not a verb-shaped slice.

## Decisions made

### 1. `UserRow.tenant_id: Option<String>`

Added to the row struct (and to `UserReversible`'s snapshot shape
by extension) with the same backwards-compat dance as
`prefs_json`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub tenant_id: Option<String>,
```

Pre-existing serialised snapshots in `starter_changes` (which had
no `tenant_id` field) deserialise as `None`, so the audit floor
holds without a backfill. New writes carry the field; absence is
indistinguishable from `None` over the wire, which is correct
because both mean "unassigned."

Ripple: 14 `UserRow { … }` literals across the user-admin verbs
and their test helpers, all mechanically updated with
`tenant_id: None`. The compiler caught every site; no silent drops
(verified via `git diff` count + `cargo build --workspace`).

### 2. FK validation — assign-time only

The verb validates that `tenant_id`, when `Some`, resolves in
[`TenantStore`](../../../crates/rubix-tools/src/tenant/store.rs)
before writing. Refusing this would let the user-admin surface
drift out of sync with the tenant directory:

> Silently assigning a user to a nonexistent tenant would be a
> footgun. Cheap one-row read; the only implementor today walks
> the full list, but production PG impls will index on id.

Implementation:

- [`TenantStore::get`](../../../crates/rubix-tools/src/tenant/store.rs)
  added with a default impl walking `list()`. Production PG
  impls override with an indexed lookup when they land.
- [`UserTenantAssignTool::invoke`](../../../crates/rubix-tools/src/user/tenant_assign.rs)
  calls `tenants.get(id)` before `users.set_tenant(...)`; an
  unknown id surfaces as `Error::NotFound { what: "tenant:..." }`
  and the user row does NOT mutate.
- The empty string is rejected as `Error::Invalid` — `Some("")`
  is almost always a wire-shaped bug (a blank form field), and
  the difference between "explicitly clear" (`null`) and
  "accidentally blank" (`""`) must stay visible at the API
  boundary. Mirrors the trim-check in `role_set`.

Unassignment (`tenant_id: null`) skips the FK check by
definition — clearing an assignment can't reference a tenant
that does not exist.

### 3. Cascade on tenant delete — out of scope, decision recorded

There is no `rubix.tenant.delete` verb today, so the question is
hypothetical for this slice. The decision is **recorded** in the
DTO module doc and the store doc so it gets debated rather than
implicitly made when a delete verb appears:

> When [a tenant-delete verb] lands, the operator-visible
> decision is whether to (a) refuse delete while users are
> assigned, (b) cascade-unassign, or (c) block at the FK.

My recommendation when it lands: **(a) refuse**. Silent cascade
is surprising; FK block (in PG) produces an opaque error message;
explicit refusal with a user-count tells the operator exactly
what to unassign first. But that's a separate slice.

### 4. Response shape — full identity echo

Mirrors `prefs_set.rs`. The response carries every
identity-bearing field so `change_for` reconstructs the snapshot
byte-exact without a follow-up store read:

```rust
pub struct UserTenantAssignResponse {
    pub summary: Diagnostic,
    pub user_id: String,
    pub email: String,
    pub prior_tenant_id: Option<String>,
    pub new_tenant_id: Option<String>,
    pub was_unchanged: bool,
    // Echoed identity-bearing fields (§3.1 bug-class avoidance):
    pub role: String,
    pub disabled_at_ms: Option<i64>,
    pub prefs_json: Option<Value>,
}
```

The regression test
`snapshot_byte_exact_preserves_role_disabled_and_prefs` pins
this: a tenant assignment on an admin-role, disabled, prefs-
bearing user produces a `before` snapshot that retains all three
non-flipped fields. Undo of that assignment will replay the
prior tenant id and **only** the prior tenant id; everything
else round-trips intact. Same load-bearing assertion shape as
the `prefs_set` session doc's `snapshot_byte_exact_preserves_role_and_disabled_state`.

While I was in the response DTOs, I also added `tenant_id` echo
to `UserDisableResponse`, `UserRoleSetResponse`, and
`UserPrefsSetResponse`. None of those verbs *touches* tenant
assignment, but their `change_for` adapters reconstruct the
full row — without the echo, undo of a role flip on a
tenant-assigned user would silently unassign them on the
`before` snapshot. Same §3.1 bug class; closed for all four
verbs in one pass.

### 5. `null` as input — `Option<String>` with `serde(default)`

```rust
#[serde(default)]
pub tenant_id: Option<String>,
```

`None` means unassign; `Some(id)` means assign. The verb
distinguishes "assign" from "unassign" in the diagnostic key
(`rubix.user.tenant.assigned` vs `rubix.user.tenant.unassigned`)
because the two read differently to an operator and the
distinction is operator-visible. Three keys total — one
unchanged path, two distinct change paths — vs the two keys
`role_set` and `prefs_set` use.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/user/tenant_assign.rs` — DTO +
  descriptor.
- `rubix/crates/rubix-tools/src/user/tenant_assign.rs` — verb,
  9 unit tests.
- `rubix/docs/sessions/undo/2026-05-28-user-tenant-assign.md`
  (this doc).

### Modified

- `rubix/crates/rubix-spi/src/dto/user/mod.rs` — `pub mod tenant_assign;`
- `rubix/crates/rubix-spi/src/dto/user/disable.rs` — `tenant_id` on response
- `rubix/crates/rubix-spi/src/dto/user/role_set.rs` — `tenant_id` on response
- `rubix/crates/rubix-spi/src/dto/user/prefs_set.rs` — `tenant_id` on response
- `rubix/crates/rubix-spi/catalogues/en.json` — three keys
  (`.assigned`, `.unassigned`, `.unchanged`)
- `rubix/crates/rubix-spi/catalogues/es.json` — three keys
- `rubix/crates/rubix-tools/src/user/mod.rs` — `pub mod tenant_assign;`
- `rubix/crates/rubix-tools/src/user/store.rs` — `tenant_id` on
  `UserRow`, `set_tenant` on `UserAdminStore` + `InMemoryUserStore`
- `rubix/crates/rubix-tools/src/user/create.rs` — `tenant_id: None`
  on the two literals (created rows are unassigned by default)
- `rubix/crates/rubix-tools/src/user/list.rs` — `tenant_id: None`
  on the test helper
- `rubix/crates/rubix-tools/src/user/disable.rs` — populate +
  echo `tenant_id` on response, full-row snapshot reconstruction
- `rubix/crates/rubix-tools/src/user/role_set.rs` — populate +
  echo `tenant_id`, full-row snapshot reconstruction
- `rubix/crates/rubix-tools/src/user/prefs_set.rs` — populate +
  echo `tenant_id`, full-row snapshot reconstruction
- `rubix/crates/rubix-tools/src/tenant/store.rs` —
  `TenantStore::get` (default impl walks `list()`)
- `rubix/crates/rubix-agent/src/registry.rs` — import + verb
  wire-in alongside the existing user-admin block
- `rubix/docs/design/undo/README.md` — §3.3 marked closed, all
  three session-doc pointers landed

## Validation

- `cargo test -p rubix-tools --lib user` → **32 passed** (was 23
  before; +9 from `tenant_assign`). All `create` / `disable` /
  `list` / `role_set` / `prefs_set` / `store` tests still green —
  the response DTO additions and `UserRow` field are wire-additive.
- `cargo test -p rubix-agent --test goal_2_user_admin_test --test
  undo_dispatch_test --test admin_registry_test --test
  admin_openapi_projection_test` → 16 total, all green. The
  registry test pin and the OpenAPI projection both pick up the
  new verb without further wiring.
- `cargo build --workspace` — clean.
- `cargo clippy -p rubix-tools -p rubix-spi --lib --tests` — only
  the two pre-existing `cleaner/registry.rs` warnings; no new
  lints from this session.

## What's closed

Proposal §3.3 — done. The audit-log proposal's six concrete
steps are now all ✅:

- ✅ #1 migration `changelog_kind_policy`
- ✅ #2 `apply_policy` helper
- ✅ #3 `boot/changelog_sweep.rs` + main.rs spawn
- ✅ #4 seed migration pinning `user`/`team` to NULL
- ✅ #5 §3.3 verb extension — three verbs landed
  (`role.set`, `prefs.set`, `tenant.assign`)
- ✅ #6 design README pointer

User-admin reversibility (proposal §3.3) is complete: every
mutating user verb writes a full-row snapshot through
`UserReversible`, audit retention is pinned at the SQL layer,
and the operator surface covers role / prefs / tenant
assignment with full undo+redo support.

## What's next

- **§3.2 node-level undo** — still gated on Phase B
  `flow_nodes`. No new blockers from this session.
- **Operator surface for `undo_kind_policy` /
  `changelog_kind_policy`** — an admin verb (`rubix.audit.policy.set`?)
  exposing what's today a migration-only knob. Check whether the
  concurrent admin-session work (in `rubix-agent/src/admin/` and
  `routes/admin/`) has stabilised before starting; the new verb
  needs to register through that same surface.
- **Warehouse-write Reversibles** — deferred until operator
  demand surfaces, per the design README's "Outstanding gaps"
  section.
- **`rubix.tenant.create` / `rubix.tenant.delete` verbs** —
  out of scope for §3.3 but the cascade decision recorded in
  this session's DTO doc should be picked up when delete lands.
