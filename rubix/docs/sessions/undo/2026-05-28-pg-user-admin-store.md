# Pg backing for the user-admin verbs (`PgUserAdminStore`)

Date: 2026-05-28
Slice: 3 of 4 (Pg backing for the rubix-tools verb surface).
Sibling sessions:
[`2026-05-28-pg-audit-policy-store.md`](2026-05-28-pg-audit-policy-store.md),
[`2026-05-28-pg-rubix-tenant-store.md`](2026-05-28-pg-rubix-tenant-store.md).

## What landed

`rubix_users` now has a Postgres backing store mirroring the
in-memory `InMemoryUserStore` byte-exact through the
`UserAdminStore` contract. The seven user-admin verbs
(`create`, `list`, `disable`, `enable`, `role.set`, `prefs.set`,
`tenant.assign`) now hit Postgres in any boot that supplies a
DSN, with the in-memory store as the no-DSN fallback. No verb
files were touched; the swap is registry-only.

## Files

Trait + row contract (moved out of `rubix-tools` so
`rubix-store-postgres` can reach it without depending on
`rubix-tools` — SCOPE R5: tools and store-postgres are
siblings, both rooted in `rubix-spi`):

- New: `rubix/crates/rubix-spi/src/user/{mod.rs, store.rs}`.
  `UserRow { user_id, email, role, disabled_at_ms,
  prefs_json, tenant_id }` (kept `PartialEq` only because
  `prefs_json: Option<serde_json::Value>` and `Value` is not
  `Eq`). `UserAdminStore` trait with 11 methods. `USER_KIND =
  "user"`.
- New: `rubix/crates/rubix-spi/src/lib.rs` registers `pub mod
  user;`.

In-memory fake updated to re-export the contract:

- Rewritten: `rubix/crates/rubix-tools/src/user/store.rs` now
  starts with `pub use rubix_spi::user::{UserAdminStore,
  UserRow, USER_KIND};` and keeps `InMemoryUserStore` +
  `UserReversible` + the two existing snapshot-shape tests
  (`create_rejects_duplicate_email`,
  `disable_is_idempotent_and_keeps_prior_timestamp`).

Pg backing:

- New migration:
  `rubix/crates/rubix-store-postgres/migrations/rubix_users/0001_create_rubix_users.sql`
  — `user_id TEXT PRIMARY KEY`, `email TEXT NOT NULL UNIQUE`,
  `role TEXT NOT NULL`, `disabled_at_ms BIGINT`,
  `prefs_json JSONB`, `tenant_id TEXT REFERENCES
  rubix_tenants(tenant_id) ON DELETE RESTRICT`, partial
  index on `tenant_id WHERE NOT NULL`.
- New impl: `rubix/crates/rubix-store-postgres/src/users/mod.rs`
  — `PgUserAdminStore` wrapping a `starter_store_postgres::pool::Pool`.
  All five mutating methods open a transaction, take
  `SELECT ... FOR UPDATE` on the prior row, detect no-ops
  (in which case both halves of the `(prior, new)` tuple match
  byte-exact per §3.1 echo rule), and otherwise UPDATE ...
  RETURNING the new row in one round-trip. `23505 ->
  Error::Conflict { message: "user with email X already exists" }`
  matches the in-memory wording byte-exact. `23503` (FK
  violation on the tenant column) maps to a clean
  `Error::Conflict` rather than leaking the raw sqlx error
  — surfaced by `set_tenant` with a missing tenant id and by
  `put` undo replay against a stale snapshot.
- `rubix/crates/rubix-store-postgres/src/lib.rs` — added
  `pub mod users;`, `pub use users::PgUserAdminStore;`,
  `RUBIX_USERS_MIGRATOR`, `RUBIX_USERS_MIGRATION_SOURCE`.

Wiring:

- `rubix/crates/rubix-agent/src/registry.rs` — import
  `PgUserAdminStore`, swap the unconditional
  `Arc::new(InMemoryUserStore::new())` to the same
  `match pg_pool { Some(pool) => Arc::new(PgUserAdminStore::new(pool.clone())),
  None => Arc::new(InMemoryUserStore::new()) }` pattern used
  by `tenant_store`, `audit_policy_store`, `dashboard_store`,
  `flow_store`.
- `rubix/crates/rubix-agent/src/boot/migrations.rs` —
  registered `RUBIX_USERS_MIGRATION_SOURCE` **after**
  `RUBIX_TENANTS_MIGRATION_SOURCE` in the source chain (FK
  dependency).

Coverage:

- New: `rubix/crates/rubix-agent/tests/user_pg_test.rs` — five
  `#[ignore]` testcontainers scenarios covering empty list,
  create + get + find_by_email round-trip, duplicate-email
  Conflict, disable/enable idempotency preserving the prior
  timestamp, set_role / set_prefs / set_tenant idempotency,
  FK Conflict on `set_tenant("does-not-exist")`, snapshot
  restore via `put`, and idempotent `delete`.

## Decisions

- **FK + RESTRICT, not CASCADE.** The verb-layer
  refuse-if-referenced check in
  `rubix-tools/src/tenant/delete.rs` is the primary
  enforcement; the DB-level RESTRICT is defense in depth. A
  silent CASCADE would unassign users without an undo entry
  and would surprise operators reading the audit log.
- **No CHECK on `role`.** Adds coupling to the enum variant
  set with no guard against the operator-visible mistake (the
  failure mode is "operator types `redaer` and the verb
  accepts it"). Deferred until a `role.add` verb makes the
  set extensible.
- **Trait moved to `rubix-spi`, not `rubix-tools`.** R5
  forbids `rubix-store-postgres` from depending on
  `rubix-tools`. Same shape as the audit-policy and
  tenant-store slices that landed earlier in the day.
- **`UserRow` stays `PartialEq` only.** `prefs_json` carries
  `serde_json::Value` and `Value: !Eq`. The no-op detection
  paths use `==` which only needs `PartialEq`.

## Gates

- `cargo build --workspace --tests` — clean.
- `cargo test -p rubix-tools --lib` — 276 / 276.
- `cargo clippy -p rubix-tools -p rubix-spi -p rubix-agent -p rubix-store-postgres --lib --tests`
  — no new warnings from this slice; pre-existing warnings
  in `rubix-agent/src/admin/...` (off-limits zone),
  `rubix-tools/src/cleaner/registry.rs`,
  `rubix-agent/src/extensions/warehouse_write.rs`, and one
  test file are unrelated.
- `cargo test -p rubix-agent --test undo_dispatch_test --test goal_2_user_admin_test --test admin_registry_test`
  — 3 + 2 + 9 green.

## Follow-ups

- Slice 4: `PgTeamAdminStore`. Open design question deferred:
  members as a JSON column on the team row vs a separate
  `rubix_team_members` join table. The verbs are
  `team.member.{assign,unassign}` which only mutate the
  `members` field; a join table buys query flexibility we do
  not need today but loses the snapshot-in-one-row property
  that makes `TeamReversible` trivial. Tentatively going with
  JSON column to mirror `prefs_json` on `rubix_users`.
- The `rubix.user.list` and `rubix.user.find_by_email`
  surfaces still scan the full table. Once the team-admin
  slice lands and we have real traffic shape data, add a
  `(tenant_id)` index conditional on observed query patterns.
