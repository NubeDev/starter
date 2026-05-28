---
date: 2026-05-28
---

# 2026-05-28 — Undo / redo: §3.3 `rubix.user.prefs.set` verb + snapshot-fidelity audit

Continues §3.3 verb extension. Ships the second of three planned
verbs (`prefs.set`), and — load-bearing for the audit story —
**fixes a §3.1-bug-class snapshot bug already present in `disable.rs`
and `role_set.rs`** that I'd let through with a "future work"
comment in the prior session.

## The bug I shipped last session

In [`2026-05-28-user-role-set.md`](2026-05-28-user-role-set.md)
I noted, in a comment, that `UserRoleSetTool::change_for`
reconstructs the snapshot with `disabled_at_ms: None` because
"the unit test pins the active-user path." That was wrong-shaped:

- The `UserReversible` snapshot is the **full** `UserRow`.
- A role change on a *disabled* user would produce a snapshot
  whose `before` has `disabled_at_ms: None`, even though the live
  row has `disabled_at_ms: Some(…)`.
- Undo of that role change would replay the `before` snapshot →
  silently **re-enable a disabled user** because the snapshot
  lost the disabled-at timestamp.

Same class of bug as the dashboard rename bug fixed in proposal
§3.1 ([prior session](2026-05-28-undo-redo-landed.md#31--dashboard-rename-undo-was-silently-broken)).
Adding `prefs_json` to `UserRow` made this worse — every
non-prefs verb's `change_for` would now also clear prefs on undo.

### Fix shape

Same pattern as `prior_title` / `prior_tags` on
`DashboardUpdateResponse`: echo every identity-bearing field on
the verb response so `change_for` reconstructs the snapshot
byte-exact without a follow-up store read. Concretely:

- [`UserDisableResponse`](../../../crates/rubix-spi/src/dto/user/disable.rs)
  gains `prefs_json: Option<Value>`. The verb populates it from
  the live row (`prior.prefs_json`); `change_for` threads it
  into both sides of the snapshot.
- [`UserRoleSetResponse`](../../../crates/rubix-spi/src/dto/user/role_set.rs)
  gains `disabled_at_ms: Option<i64>` **and** `prefs_json`. Same
  shape: populated from live row, used on both snapshot sides
  (role_set doesn't touch either).
- [`UserPrefsSetResponse`](../../../crates/rubix-spi/src/dto/user/prefs_set.rs)
  carries `role` and `disabled_at_ms` from day one, learning from
  the bug.

Wire-compat impact: all three additions are `Option<…>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]` so
existing clients that don't know about the new fields read them
as `None` and write payloads that round-trip cleanly.

### Regression-pinning test

New unit test in
[`user/prefs_set.rs`](../../../crates/rubix-tools/src/user/prefs_set.rs#L242-L270)
— `snapshot_byte_exact_preserves_role_and_disabled_state` —
seeds an admin-role + disabled user, runs `prefs.set`, asserts
both snapshot sides retain `role: "admin"` and the disabled
timestamp. This is the load-bearing assertion for the
audit-floor story: an audit row that lies about the user's
state at the time of the write is worse than no audit row at
all.

`role_set.rs` and `disable.rs` carry equivalent assertions in
their existing `change_for_records_…` tests; the prefs round-trip
is the new failure mode this turn closes.

## `rubix.user.prefs.set` — the v2 slice

### DTO

[`rubix-spi/src/dto/user/prefs_set.rs`](../../../crates/rubix-spi/src/dto/user/prefs_set.rs)
— `UserPrefsSetRequest { user_id?, email?, prefs: Value }` +
`UserPrefsSetResponse { summary, user_id, email, prior_prefs,
new_prefs, was_unchanged, role, disabled_at_ms }`.

Notable shape decisions:

- `prefs: Value` is **required** in the request; opaque JSON. The
  rubix tools do not interpret the blob — the UI / agent loop
  does. Consistent with the
  [`i18n_and_unit_prefs`](../../../../../home/user/.claude/projects/-home-user-code-rust-starter/memory/i18n_and_unit_prefs.md)
  memory which says prefs content (locale, units) is
  per-user-configurable; the storage shape is orthogonal.
- `null` is legal input and stored as `Some(Value::Null)` — a
  semantic distinct from `None` ("no prefs row ever set"). The
  unit test
  `null_prefs_is_legal_and_stored_as_some_null` pins this; a
  future "clear prefs back to `None`" verb can land if the
  distinction matters operationally.
- `prior_prefs: Option<Value>` because the prior state may be
  "no row" (`None`). Always present in the response when
  `was_unchanged = false`.

### Store

Extended [`UserAdminStore`](../../../crates/rubix-tools/src/user/store.rs)
with one new method `set_prefs(user_id, prefs) -> (UserRow,
UserRow)`. Idempotent: byte-equal `prior.prefs_json == new` short-
circuits without mutating. Mirrors `set_role` and `disable`.

`UserRow` itself grew a new field
`prefs_json: Option<Value>` with `#[serde(default,
skip_serializing_if = "Option::is_none")]`. Pre-existing
serialised snapshots (which had no `prefs_json` field)
deserialise as `None` — backwards compatible at the storage
layer.

### Verb

[`rubix-tools/src/user/prefs_set.rs`](../../../crates/rubix-tools/src/user/prefs_set.rs)
— `UserPrefsSetTool` implementing `Tool` + `ReversibleTool`. Six
unit tests:

1. `set_prefs_on_blank_row_changes_none_to_some` — happy path.
2. `set_same_prefs_is_noop_and_skips_draft` — idempotency;
   `change_for → None` on no-op (matches `disable` / `role_set`).
3. `change_for_snapshot_carries_full_row_with_prior_prefs` —
   pins that the `before` snapshot has the *prior* prefs blob,
   not `None` (load-bearing).
4. `snapshot_byte_exact_preserves_role_and_disabled_state` —
   the bug-class regression test described above.
5. `missing_target_returns_not_found` — id-resolution path.
6. `null_prefs_is_legal_and_stored_as_some_null` — pins the
   None-vs-Some(Null) semantic.

All 6 green.

### Diagnostic catalogue

Two new keys in
[`rubix-spi/catalogues/en.json`](../../../crates/rubix-spi/catalogues/en.json)
and
[`es.json`](../../../crates/rubix-spi/catalogues/es.json):

- `rubix.user.prefs.set` — "Prefs updated for user {email} (at {at})."
- `rubix.user.prefs.unchanged` — "User {email} prefs unchanged; no change made (at {at})."

Kept the prefs blob out of the diagnostic params on purpose — a
locale change message saying "prefs updated for ada@x" is more
useful than one that dumps the JSON. Operators can pull the
delta from the audit row if they need it.

### Registry

One new import + one `wrap_rev(Arc::new(UserPrefsSetTool::new(…)))`
line in [`registry.rs`](../../../crates/rubix-agent/src/registry.rs)
under the `// ---- user admin ----` section. As with `role_set`,
the existing `UserReversible` registration for `USER_KIND` picks
up the new verb's `change_for` adapter automatically.

## Validation

- `cargo build --workspace` — clean.
- `cargo test -p rubix-tools --lib user` → `23 passed` (was 17
  before; +6 from `prefs_set`). All `create` / `disable` /
  `list` / `role_set` / `store` tests still green — the response
  DTO additions are wire-additive only.
- `cargo test -p rubix-agent --test goal_2_user_admin_test --test
  undo_dispatch_test --test admin_registry_test --test
  admin_openapi_projection_test` → 16 total, all green.
- `cargo clippy -p rubix-tools -p rubix-spi --lib --tests` — same
  two pre-existing warnings in `cleaner/registry.rs`, no new
  lints from this session.

## What's left for §3.3

The audit-log proposal's six concrete steps are now:

- ✅ #1 migration `changelog_kind_policy`
- ✅ #2 `apply_policy` helper
- ✅ #3 `boot/changelog_sweep.rs` + main.rs spawn
- ✅ #4 seed migration pinning `user`/`team` to NULL
- 🟡 #5 §3.3 verb extension — **`role.set` + `prefs.set` landed;
  `tenant.assign` outstanding**
- ✅ #6 design README pointer

Only `rubix.user.tenant.assign` remains. Bigger model change
than the other two:

- `UserRow` has no `tenant_id` field today.
- `tenant` is its own kind with its own store; no link to
  `UserRow` exists.
- An FK / referential decision is needed (does assignment
  validate the tenant exists? does deleting a tenant cascade
  to unassign its users? — both are operator-visible).

Treat as a separate session — it's a model change with ripple
impact, not a verb-shaped slice.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/user/prefs_set.rs`
- `rubix/crates/rubix-tools/src/user/prefs_set.rs`
- `rubix/docs/sessions/undo/2026-05-28-user-prefs-set.md` (this doc)

### Modified

- `rubix/crates/rubix-spi/src/dto/user/mod.rs` — `pub mod prefs_set;`
- `rubix/crates/rubix-spi/src/dto/user/disable.rs` — `prefs_json` on response
- `rubix/crates/rubix-spi/src/dto/user/role_set.rs` — `disabled_at_ms` + `prefs_json` on response
- `rubix/crates/rubix-spi/catalogues/en.json` — two keys
- `rubix/crates/rubix-spi/catalogues/es.json` — two keys
- `rubix/crates/rubix-tools/src/user/mod.rs` — `pub mod prefs_set;`
- `rubix/crates/rubix-tools/src/user/store.rs` — `prefs_json` on
  `UserRow`, `set_prefs` on `UserAdminStore` + `InMemoryUserStore`
- `rubix/crates/rubix-tools/src/user/create.rs` — `prefs_json: None`
  on the two literals
- `rubix/crates/rubix-tools/src/user/list.rs` — `prefs_json: None`
  on the test helper
- `rubix/crates/rubix-tools/src/user/disable.rs` — populate +
  echo `prefs_json`, full-row snapshot reconstruction
- `rubix/crates/rubix-tools/src/user/role_set.rs` — populate +
  echo `disabled_at_ms` + `prefs_json`, full-row snapshot
  reconstruction; comment now explains the §3.1 bug class instead
  of deferring it
- `rubix/crates/rubix-agent/src/registry.rs` — import + verb wire-in
