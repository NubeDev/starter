# Pg backing for the team-admin verbs (`PgTeamAdminStore`)

Date: 2026-05-28
Slice: 4 of 4 (Pg backing for the rubix-tools verb surface).
Sibling sessions:
[`2026-05-28-pg-audit-policy-store.md`](2026-05-28-pg-audit-policy-store.md),
[`2026-05-28-pg-rubix-tenant-store.md`](2026-05-28-pg-rubix-tenant-store.md),
[`2026-05-28-pg-user-admin-store.md`](2026-05-28-pg-user-admin-store.md).

## What landed

`rubix_teams` now has a Postgres backing store mirroring the
in-memory `InMemoryTeamStore` byte-exact through the
`TeamAdminStore` contract. The five team verbs
(`create`, `update`, `delete`, `member.assign`,
`member.unassign`) now hit Postgres in any boot that supplies a
DSN, with the in-memory store as the no-DSN fallback. No verb
files were touched; the swap is registry-only. This closes the
Pg-backing-for-rubix-tools work \u{2014} the four
operator-visible stores (audit policy, tenants, users, teams)
all now have a Pg path.

## Files

Trait + row contract (moved out of `rubix-tools` so
`rubix-store-postgres` can reach it without depending on
`rubix-tools` \u{2014} SCOPE R5: tools and store-postgres are
siblings, both rooted in `rubix-spi`):

- New: `rubix/crates/rubix-spi/src/team/{mod.rs, store.rs}`.
  `TeamRow { team_id, name, description, members:
  BTreeMap<String, i64> }` (gets `Eq` because `BTreeMap<String,
  i64>: Eq`). `TeamPatch { members, name, description }` (the
  sparse payload `TeamReversible` reads / writes).
  `TeamAdminStore` trait with 8 methods. `TEAM_KIND = "team"`.
- `rubix/crates/rubix-spi/src/lib.rs` registers `pub mod team;`.

In-memory fake updated to re-export the contract:

- Rewritten: `rubix/crates/rubix-tools/src/team/store.rs` now
  starts with `pub use rubix_spi::team::{TeamAdminStore,
  TeamPatch, TeamRow, TEAM_KIND};` and keeps
  `InMemoryTeamStore` + `TeamReversible` + `merge_patch` +
  `parse_patch` + `parse_row` helpers + the existing
  `assign_is_idempotent_and_keeps_prior_timestamp` test.
  No verb file was touched (the existing `use crate::team::store::{...}`
  imports keep resolving via the re-export).

Pg backing:

- New migration:
  `rubix/crates/rubix-store-postgres/migrations/rubix_teams/0001_create_rubix_teams.sql`
  \u{2014} `team_id TEXT PRIMARY KEY`, `name TEXT NOT NULL
  UNIQUE`, `description TEXT`, `members JSONB NOT NULL DEFAULT
  '{}'::jsonb`. GIN index on `members` for the future
  "list-teams-containing-user" report. Long preamble in the
  SQL captures the join-table-vs-JSONB design choice.
- New impl: `rubix/crates/rubix-store-postgres/src/teams/mod.rs`
  \u{2014} `PgTeamAdminStore` wrapping a
  `starter_store_postgres::pool::Pool`. `assign` and `unassign`
  both open a transaction, take `SELECT ... FOR UPDATE` on the
  prior row, detect membership-no-op (in which case both halves
  of `(prior, new)` match byte-exact per \u{00A7}3.1 echo
  rule), and otherwise mutate the `members` JSONB column in
  one round-trip. `23505 -> Error::Conflict { message: "team
  with name X already exists" }` matches the in-memory
  wording byte-exact. `delete` returns `NotFound` on
  `rows_affected() == 0` (matches the in-memory fake \u{2014}
  the verb relies on this signal).
- `rubix/crates/rubix-store-postgres/src/lib.rs` \u{2014} added
  `pub mod teams;`, `pub use teams::PgTeamAdminStore;`,
  `RUBIX_TEAMS_MIGRATOR`, `RUBIX_TEAMS_MIGRATION_SOURCE`.

Wiring:

- `rubix/crates/rubix-agent/src/registry.rs` \u{2014} import
  `PgTeamAdminStore`, swap the unconditional
  `Arc::new(InMemoryTeamStore::new())` to the same
  `match pg_pool` pattern used by every other rubix store.
- `rubix/crates/rubix-agent/src/boot/migrations.rs` \u{2014}
  registered `RUBIX_TEAMS_MIGRATION_SOURCE` in the source
  chain. No FK dependency so position is independent;
  placed adjacent to `RUBIX_USERS_MIGRATION_SOURCE` for
  readability.

Coverage:

- New: `rubix/crates/rubix-agent/tests/team_pg_test.rs` \u{2014}
  four `#[ignore]` testcontainers scenarios covering empty
  list / create + get / duplicate-name Conflict;
  assign / unassign idempotency preserving `assigned_at_ms`;
  NotFound on missing-team for assign / unassign / delete;
  snapshot restore via `put` with a populated members map +
  redundant-delete NotFound.

## Decisions

- **Members as JSONB on the team row, NOT a separate join
  table.** Three reasons (recorded in the migration
  preamble):
  1. `TeamReversible` is snapshot-shaped against the full
     row \u{2014} a join table would lose the
     single-row-snapshot property and force every
     `member.assign` / `member.unassign` to write the change
     AND mutate a sibling table in the same transaction.
  2. Members never exceed Pg's TOAST page in any realistic
     team.
  3. We never read "all teams a user belongs to" from a
     write path; the future user-detail report can use the
     `members ? user_id` jsonb operator over the new GIN
     index.
- **No FK from members keys into `rubix_users.user_id`.**
  JSONB keys can't carry FKs. The `rubix.user.delete` verb
  (when it lands) will walk teams and unassign before
  deleting; the join-table alternative would just push the
  same logic into a deferred FK that crashes the
  transaction far from the originating verb. Same cascade
  posture as the tenant FK from `rubix_users` \u{2014} the
  verb is the primary enforcement, the DB is defense in
  depth where the JSONB shape allows.
- **Conflict message matches the in-memory fake byte-exact
  regardless of which constraint fired.** Both PK
  (`team_id`) and UNIQUE (`name`) collisions report "team
  with name X already exists" because the in-memory fake
  only checks the name; id collisions are an upstream
  id-generation bug, not an operator-visible conflict.
- **`delete` returns `NotFound` on missing rows.** Unlike
  `users` (where redundant-delete is a no-op for undo
  replay), the team-delete verb relies on the `NotFound`
  signal to distinguish a missing-target call from a
  successful no-op. The trait contract was already this
  way for the in-memory fake; Pg matches.
- **GIN index on `members`** \u{2014} accepted minor cost
  on inserts to keep the future user-detail-page report
  sub-millisecond.

## Gates

- `cargo build --workspace --tests` \u{2014} clean.
- `cargo test -p rubix-tools --lib` \u{2014} 276 / 276.
- `cargo clippy -p rubix-spi -p rubix-tools -p rubix-store-postgres --lib --tests`
  \u{2014} no new warnings from this slice; pre-existing
  warnings in `rubix-tools/src/cleaner/registry.rs`
  unrelated.
- `cargo test -p rubix-agent --test undo_dispatch_test --test goal_2_user_admin_test --test admin_registry_test`
  \u{2014} 3 + 2 + 9 green.

## Follow-ups

- The four-slice Pg-backing arc is complete. Remaining
  rubix-store work moves to feature-driven slices (cross-team
  reports, retention sweeps, the future `rubix.user.delete`
  verb that has to walk teams).
- `rubix.user.delete` (when it lands) needs to walk teams via
  the new `members ? user_id` jsonb operator and unassign
  before deleting. The GIN index lands today so the walk is
  free.
- Consider extracting the `(begin / FOR UPDATE / no-op-detect /
  UPDATE / commit)` pattern into a helper on
  `starter_store_postgres::pool::Pool` if it shows up in a
  fifth store. Three impls (`PgUserAdminStore`,
  `PgAuditPolicyStore`, `PgTeamAdminStore`) is the threshold
  to watch; not refactoring yet because each has slightly
  different prior-row shape.
