---
date: 2026-05-28
---

# 2026-05-28 — Undo / redo: §3.3 `rubix.user.role.set` verb

Closes the first slice of proposal §3.3 — the role-change verb the
audit-log proposal called out as the motivating example ("I just
demoted the wrong person"). Per the parent proposal's strategy of
keeping landing slices small and security-relevant first, this
session ships **only** `rubix.user.role.set`; `prefs.set` and
`tenant.assign` follow the same template in a future session.

## What landed

### DTO

[`rubix-spi/src/dto/user/role_set.rs`](../../../crates/rubix-spi/src/dto/user/role_set.rs)
— `UserRoleSetRequest { user_id?, email?, role }` + `UserRoleSetResponse
{ summary, user_id, email, prior_role, new_role, was_unchanged }` +
descriptor. Mirrors the shape of
[`disable.rs`](../../../crates/rubix-spi/src/dto/user/disable.rs)
including the `was_unchanged` idempotency flag.

`prior_role` echoed on the response so the `change_for` adapter does
not need a follow-up store read to populate the `before` snapshot —
same pattern as `prior_title` / `prior_tags` on dashboard responses
([§3.1 landing](2026-05-28-undo-redo-landed.md#31--dashboard-rename-undo-was-silently-broken)).

### Store

Extended [`UserAdminStore`](../../../crates/rubix-tools/src/user/store.rs)
with one new method:

```rust
async fn set_role(&self, user_id: &str, role: &str) -> Result<(UserRow, UserRow)>;
```

Idempotent: same-role call returns `(prior, prior)`. Mirrors the
`disable` contract. `InMemoryUserStore` impl is straight-line
HashMap mutation; production PG impl follows when the PG-backed
store lands.

### Verb

[`rubix-tools/src/user/role_set.rs`](../../../crates/rubix-tools/src/user/role_set.rs)
— `UserRoleSetTool` implementing `Tool` + `ReversibleTool`. Six
unit tests, all green:

1. `set_role_changes_prior_to_new` — happy path emits
   `rubix.user.role.set` with `prior_role != new_role`.
2. `set_same_role_is_noop_and_skips_draft` — idempotency: emits
   `rubix.user.role.unchanged`, `change_for` returns `None` so
   undo cannot accidentally re-apply the same role.
3. `change_for_records_update_with_before_after_snapshots` — pins
   the contract that the snapshot is the full `UserRow`, not a
   delta (`user_id`, `email`, `disabled_at_ms` all round-trip
   alongside the role flip).
4. `empty_role_is_rejected` — `Error::Invalid` at the entry
   guard.
5. `untrimmed_role_is_rejected` — same guard; protects role
   taxonomy from `" admin "` vs `"admin"` drift.
6. `missing_target_returns_not_found` — id-resolution error path.

### Diagnostic catalogue

Two new keys in
[`rubix-spi/catalogues/en.json`](../../../crates/rubix-spi/catalogues/en.json)
and
[`es.json`](../../../crates/rubix-spi/catalogues/es.json):

- `rubix.user.role.set` — "User {email} role changed from {prior} to {new} (at {at})."
- `rubix.user.role.unchanged` — "User {email} already had role {new}; no change made (at {at})."

Spanish translations included per the project's EN/ES policy
([`i18n_and_unit_prefs`](../../../../.../../../../home/user/.claude/projects/-home-user-code-rust-starter/memory/i18n_and_unit_prefs.md)).

### Registry wire-in

[`rubix-agent/src/registry.rs`](../../../crates/rubix-agent/src/registry.rs)
— one new import + one `wrap_rev(Arc::new(UserRoleSetTool::new(…)))`
line under the `// ---- user admin ----` section. `UserReversible`
is already registered for `USER_KIND`, so the new verb's
`change_for` is picked up by the existing dispatcher with no extra
plumbing.

## Why split the audit-floor work from the verb landing

The §3.3 work could have been compressed into one session by
landing `prefs.set` and `tenant.assign` alongside `role.set`. I
held off because:

- `prefs_json` requires a schema decision (free-form JSON vs.
  enum-validated `prefs::{Locale, Units, …}`) tied to the
  [`i18n_and_unit_prefs`](../../../../home/user/.claude/projects/-home-user-code-rust-starter/memory/i18n_and_unit_prefs.md)
  memory — wants a separate design pass.
- `tenant.assign` requires a new column on `UserRow` (no
  `tenant_id` today). Touching `UserRow` shape ripples through
  every existing user-admin test — better in its own slice.
- `role.set` is the named security example in the proposal; it's
  also the smallest, most independent change. Landing it alone
  proves the audit-floor mechanism is correctly wired end-to-end
  (every `Change` row this verb produces now lands in
  `starter_changes` against the `user` kind, which the seeded
  `changelog_kind_policy.user → NULL` row pins to keep forever).

## Validation

- `cargo build --workspace` — clean.
- `cargo test -p rubix-tools --lib user::role_set` → `6 passed`.
- `cargo test -p rubix-tools --lib user` → `17 passed` (no
  regression in `create`, `disable`, `list`, or `store` tests).
- `cargo test -p rubix-agent --test goal_2_user_admin_test --test
  undo_dispatch_test` → `9 / 9` total, no regressions in the
  end-to-end user-admin smoke or the undo-dispatch invariants.
- `cargo test -p rubix-agent --test admin_registry_test --test
  admin_openapi_projection_test` → 9 passed — the new verb did not
  break the admin registry surface or the openapi projection.
- `cargo clippy -p rubix-tools -p rubix-spi --lib --tests` —
  warnings present are all pre-existing in `cleaner/registry.rs`;
  no new lints from this session.

## What's left for §3.3

The audit-log proposal's six concrete steps are now:

- ✅ #1 migration `changelog_kind_policy`
- ✅ #2 `apply_policy` helper
- ✅ #3 `boot/changelog_sweep.rs` + main.rs spawn
- ✅ #4 seed migration pinning `user`/`team` to NULL
- 🟡 #5 §3.3 verb extension — **`rubix.user.role.set` landed; `prefs.set`
  and `tenant.assign` outstanding**
- ✅ #6 design README pointer

Two remaining verbs, both following the `role_set.rs` template:

- **`rubix.user.prefs.set`** — needs the `prefs_json: Option<Value>`
  field on `UserRow` and a decision on whether prefs are
  free-form JSON or a typed struct. Tie-in with the
  `i18n_and_unit_prefs` memory.
- **`rubix.user.tenant.assign`** — needs a new `tenant_id:
  Option<String>` column on `UserRow` and an FK / referential
  decision (today `tenant` is its own kind with its own store; no
  link to `UserRow` exists). Bigger model change than `role.set`.

Either is a clean follow-up; `role.set` is the proof of life for
the whole audit-floor + reversible-verb pipeline.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/user/role_set.rs`
- `rubix/crates/rubix-tools/src/user/role_set.rs`
- `rubix/docs/sessions/undo/2026-05-28-user-role-set.md` (this doc)

### Modified

- `rubix/crates/rubix-spi/src/dto/user/mod.rs` — `pub mod role_set;`
- `rubix/crates/rubix-spi/catalogues/en.json` — two keys
- `rubix/crates/rubix-spi/catalogues/es.json` — two keys
- `rubix/crates/rubix-tools/src/user/mod.rs` — `pub mod role_set;`
- `rubix/crates/rubix-tools/src/user/store.rs` — `set_role` on
  `UserAdminStore` + `InMemoryUserStore` impl
- `rubix/crates/rubix-agent/src/registry.rs` — import + verb wire-in
