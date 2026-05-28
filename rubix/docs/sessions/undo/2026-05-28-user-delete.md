# `rubix.user.delete` \u{2014} hard-delete verb with refuse-if-in-teams cascade

Date: 2026-05-28
Slice: post-Pg-arc. Follows the four Pg-backing slices
([audit](2026-05-28-pg-audit-policy-store.md),
[tenant](2026-05-28-pg-rubix-tenant-store.md),
[user](2026-05-28-pg-user-admin-store.md),
[team](2026-05-28-pg-team-admin-store.md)) which laid the
groundwork: cross-store walk over users + teams is now stable
on both in-memory and Postgres.

## What landed

`rubix.user.delete` is the canonical hard-delete verb for users.
Flagged as the natural next slice in three prior session docs
([audit-policy](2026-05-28-audit-policy.md),
[user-enable](2026-05-28-user-enable.md),
[pg-team-admin-store](2026-05-28-pg-team-admin-store.md))
with the cascade decision pre-committed: refuse-if-referenced
mirror of [`rubix.tenant.delete`](2026-05-28-tenant-lifecycle.md).
This slice closes the gap.

Operator-visible: GDPR / right-to-be-forgotten requests, typo
cleanup, staging account purge. Most workflows still want
`rubix.user.disable` (which keeps the row for audit and is
single-step-undoable via `rubix.user.enable`); the descriptor
and `when_not_to_use` spell out the distinction.

## Files

DTO:

- New: `rubix/crates/rubix-spi/src/dto/user/delete.rs`.
  `UserDeleteRequest { user_id?, email? }` (mirrors disable's
  resolve-by-either posture), `UserDeleteResponse` carrying
  every identity-bearing field on the deleted row
  (\u{00A7}3.1 echo rule \u{2014} required so
  `change_for` can reconstruct the full prior snapshot
  without re-reading the now-deleted row).
  `REQUIRED_PERMISSION = "users.write"` (shared scope today;
  the doc records a future `users.delete` scope as an
  open question rather than implicitly adopting it).
- `rubix/crates/rubix-spi/src/dto/user/mod.rs` registers
  `pub mod delete;`.

Verb:

- New: `rubix/crates/rubix-tools/src/user/delete.rs`.
  `UserDeleteTool` wraps `Arc<dyn UserAdminStore>` +
  `Arc<dyn TeamAdminStore>` (mirrors
  `TenantDeleteTool`'s users+tenants pair).
  - Resolves target via id or email, returns structured
    `NotFound` on miss.
  - Cascade check: walks `team_store.list()` and counts teams
    where `members.contains_key(&user_id)`. On refuse,
    surfaces `rubix.user.in_teams` diagnostic with `count`
    and `teams` (preview of first 10 names) so the operator
    can run `rubix.team.list` / `rubix.team.member.unassign`
    explicitly.
  - On success, deletes via the store and returns the full
    prior snapshot in the response.
  - `change_for` reconstructs `Op::Delete` with full-row
    `before` from the response \u{2014} no store re-read
    (the row no longer exists).
- `rubix/crates/rubix-tools/src/user/mod.rs` registers
  `pub mod delete;`.

Wiring:

- `rubix/crates/rubix-agent/src/registry.rs` \u{2014}
  imports `UserDeleteTool`, registers it next to
  `UserDisableTool` with `wrap_rev(...)` so it participates
  in the undo dispatcher.

Coverage:

- Four unit tests under `user::delete::tests` (all
  in-memory \u{2014} the cross-store walk it exercises is
  Pg-validated end-to-end via the existing
  `user_pg_test.rs` + `team_pg_test.rs` suites that proved
  `list()` and `delete()` round-trip cleanly):
  1. `delete_unassigned_user_succeeds` \u{2014} happy path,
     diagnostic code `rubix.user.deleted`.
  2. `delete_refuses_when_user_in_any_team` \u{2014} cascade
     refuse, asserts both team names appear in the
     diagnostic params, asserts the user row is still
     present afterwards.
  3. `delete_missing_user_returns_not_found` \u{2014}
     `Error::NotFound` on miss before the cascade check
     runs.
  4. `change_for_echoes_full_prior_snapshot` \u{2014}
     mutates the seeded row through `set_role` + `set_prefs`
     + `disable` first, then asserts the recorded `before`
     captures every field byte-exact. Pins the \u{00A7}3.1
     echo rule for this verb.

Also fixed a pre-existing clippy warning surfaced during the
gate run: `users/mod.rs:175` had a redundant
`UserRow::from(into_user_row(&prior))` (the helper already
returns `UserRow`). Removed the no-op conversion.

## Decisions

- **Refuse-if-in-teams cascade, not cascade-unassign.**
  Mirrors `tenant.delete`. The DTO doc enumerates the
  alternatives and why each was rejected:
  - Cascade-unassign would produce N audit entries that
    surprise the operator later, and would let an operator
    delete-then-recreate to forcibly unassign users from
    teams they don't own.
  - Block-at-FK is not available because team membership is
    JSONB (no FK target).
  - Refuse-with-diagnostic gives the operator a structured
    `rubix.user.in_teams` payload they can act on.
- **Tenant assignment is NOT a refuse condition.** Unlike
  team membership (which lives on the team row), the user's
  `tenant_id` is a column on the user row itself \u{2014}
  it disappears with the row on delete. Undo restores the
  assignment byte-exact via the snapshot. Recorded in the
  DTO doc so future readers don't add a misguided
  refuse-if-tenant-assigned check.
- **Resolve by user_id or email** (same posture as
  `disable` / `enable`). The DTO comment says "passing both
  is accepted; user_id wins" \u{2014} consistent with
  `resolve_target`'s implementation.
- **Cap the echoed teams list at 10 names** in the
  diagnostic, keep `count` authoritative. A user on
  hundreds of teams (operator account) would otherwise
  produce an unreadable error payload. Operator can pivot
  to `rubix.team.list` for the full list.
- **`users.write` permission, not a new `users.delete`
  scope.** Shared lifecycle scope is consistent with the
  other verbs today. A future `users.delete` scope (split
  for DPO role) is recorded as a follow-up rather than
  silently introduced.

## Gates

- `cargo build --workspace --tests` \u{2014} clean.
- `cargo test -p rubix-tools --lib` \u{2014} **280 / 280**
  (was 276; +4 from the new delete verb tests).
- `cargo clippy -p rubix-spi -p rubix-tools -p
  rubix-store-postgres -p rubix-agent --lib --tests`
  \u{2014} no new warnings; the one I introduced in the
  prior slice (`UserRow::from(into_user_row(...))`) was
  fixed here.
- `cargo test -p rubix-agent --test undo_dispatch_test
  --test goal_2_user_admin_test --test admin_registry_test`
  \u{2014} 3 + 2 + 9 green.

Pg integration tests for the four backing stores all 11/11
green on real testcontainers Postgres (verified live this
session before starting the verb work):

```text
audit_policy_pg_test  1/1
tenant_pg_test        1/1
user_pg_test          5/5
team_pg_test          4/4
```

## Follow-ups

- **Cross-store integration test** \u{2014} a future
  `user_delete_pg_test.rs` would seed a Pg user + team
  with the user as a member, exercise the cascade refuse,
  unassign, then complete the delete. Skipped today because
  the underlying stores are individually Pg-validated and
  the verb's logic is fully in-process (no Pg-specific
  branch). Add when a regression makes it worth the
  testcontainers cost.
- **`users.delete` permission split** \u{2014} a DPO role
  scoped to GDPR deletions (separate from the operator
  `users.write` scope that lets tenant admins disable
  users). Recorded in the DTO doc.
- **`rubix.tenant.delete` could use the same teams check**
  if memberships ever start carrying tenant scoping; not
  needed today because team membership is global.
