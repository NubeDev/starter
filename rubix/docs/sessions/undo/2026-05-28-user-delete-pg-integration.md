# Session 2026-05-28 — user.delete cross-store Pg integration test

Follow-up slice that closes the highest-confidence gap left by
[`2026-05-28-user-delete.md`](2026-05-28-user-delete.md): the
`UserDeleteTool` cascade walks two stores (`UserAdminStore` +
`TeamAdminStore`) but the unit tests run both against in-memory
fakes. This slice adds the cross-store integration test against
real Postgres.

## What landed

New test file
[`rubix/crates/rubix-agent/tests/user_delete_pg_test.rs`](../../crates/rubix-agent/tests/user_delete_pg_test.rs):
one `#[ignore]`-gated scenario, `user_delete_cross_store_against_postgres`,
that spins testcontainers Postgres, applies the three rubix
migrations (`rubix_tenants`, `rubix_users`, `rubix_teams`),
wires `PgUserAdminStore` + `PgTeamAdminStore` into
`UserDeleteTool::new`, and exercises:

1. **Unassigned user deletes cleanly** — response carries the
   full prior `UserRow` (id, email, role, disabled_at_ms,
   prefs_json, tenant_id all echoed per the §3.1 echo rule),
   subsequent `get` returns `None`.
2. **User in a team — refused.** `Error::Conflict` whose
   message is the structured `rubix.user.in_teams`
   diagnostic. Asserts on three things in the JSON payload:
   the diagnostic key, the blocking team name (`"Ops"`), and
   the user identity. User row is still present after the
   refuse.
3. **Drain + retry succeeds.** Unassigns the user from the
   team, retries the delete (this time by `email` to cover
   both resolve paths), then asserts: user row gone, team
   row preserved with `members.is_empty()`.

The scenario goes through `Tool::invoke` (not the store
methods directly) so it locks the verb contract end-to-end:
JSON request shape, JSON response shape, conflict-payload
shape, and the cross-store cascade walking the JSONB
`members` column on the team row.

## Why now (and not at slice time)

The user-delete session doc deferred this test on the
argument that the underlying stores are individually
Pg-validated and the verb has no Pg-specific branch. That
argument is sound but the test cost is low (one tokio test,
~2s including Pg startup) and the failure modes it catches
are non-obvious:

- A `team_store.list()` shape mismatch where the JSONB
  `members` column deserializes differently from the
  in-memory `BTreeMap<String, i64>` would let a user pass
  the cascade check even while assigned.
- A `NotFound` variant drift between the Pg store and the
  in-memory fake would change the `resolve_target` error
  the verb returns.
- The team-row `members.is_empty()` assertion after the
  retry covers the "delete must not cascade through teams"
  invariant — if a future refactor accidentally drops the
  team row when its last member is deleted, this test
  catches it.

## Decisions

- **Scenario shape mirrors the unit tests deliberately.**
  Same three cases (clean delete, refused-in-team,
  drain-and-retry); the only delta is the underlying stores
  and that scenario 3 resolves by `email` to also cover
  `find_by_email` against Pg in the delete path.
- **Assert on diagnostic JSON substrings, not localized
  strings.** The diagnostic is serialized as JSON in the
  conflict message; we check for the key
  (`"rubix.user.in_teams"`), the team name, and the user
  identity. Locks the structured payload without coupling
  to message-key string formatting that the i18n layer
  could legitimately change.
- **Single `assigned_at_ms = 1_700_000_000_000`** for the
  team membership — the test never inspects it; using a
  fixed sentinel keeps the scenario deterministic and
  documents that it's a "don't care" value.
- **Did NOT add a follow-up scenario for the `rubix.user.in_teams`
  count-vs-preview cap (10 teams).** The unit tests can
  reach that boundary far cheaper. The Pg test exists to
  catch cross-store wiring drift, not redundant
  diagnostic-shaping coverage.

## Gates

- `cargo build --workspace` — clean.
- `cargo clippy -p rubix-tools -p rubix-spi -p rubix-agent
  --lib --tests` — no new warnings (pre-existing
  `items_after_test_module` on `_table_column_anchor` in
  the dashboard-row glue is unrelated).
- `cargo test -p rubix-tools --lib` — **280 / 280** (no
  delta; this slice adds only an integration test).
- `cargo test -p rubix-agent --test undo_dispatch_test
  --test goal_2_user_admin_test --test admin_registry_test`
  — 3 + 2 + 9 green.
- `cargo test -p rubix-agent --test user_delete_pg_test --
  --ignored` — **1 / 1** green on real testcontainers
  Postgres.

Live Pg suite total now **12 / 12** (was 11 / 11):

```text
audit_policy_pg_test       1/1
tenant_pg_test             1/1
user_pg_test               5/5
team_pg_test               4/4
user_delete_pg_test        1/1   (new this slice)
```

## Follow-ups

Unchanged from the user-delete slice:

- **`users.delete` permission split** — still gated on a
  DPO-role driver. Not introduced this slice.
- **`rubix.tenant.delete` teams check** — still gated on
  team memberships gaining tenant scoping.
- **Pg store shared FOR UPDATE helper** — still below the
  abstraction threshold (4 sites, varying prior-row
  types).

Nothing new opened by this slice.
