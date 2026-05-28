# Session 2026-05-28 — `PgRubixTenantStore`

## Slice

Shipped the Postgres-backed `TenantStore` so the
`rubix.tenant.{create,update,list,delete}` and
`rubix.user.tenant.assign` verbs persist their tenant directory
across restarts when `pg_pool` is available. Mirrors the
just-landed [`PgAuditPolicyStore`](./2026-05-28-pg-audit-policy-store.md)
slice: trait + row moved to `rubix-spi::tenant`, in-memory fake
stays in `rubix-tools`, Pg impl lives in `rubix-store-postgres`,
registry selects via `match pg_pool`. First slice of the brief
that **introduces a new rubix-owned table**, so it also adds a
new migration source to the boot chain.

## Decisions

### New migration source: `rubix_tenants`

No prior table existed for the verb-surface tenant directory.
The auth-side `tenants` table owned by `starter-auth-users` is a
separate concept (canonical for login) and the rubix-side module
docs already declare them intentionally split. New schema:

```sql
CREATE TABLE IF NOT EXISTS rubix_tenants (
    tenant_id TEXT PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    locale    TEXT NOT NULL DEFAULT 'en'
);
```

- `tenant_id` as TEXT (not UUID) so the bundled `"system"` row
  the SDUI pages reference stays readable in `psql` and
  shareable across the in-memory and Pg-backed boot paths
  byte-exact.
- `name` carries a `UNIQUE` constraint so the Pg impl's `create`
  does not have to second-walk the table the way the in-memory
  impl does. The trait contract requires uniqueness on both id
  AND name and the Pg layer leans on the database to enforce
  it.
- No timestamps. The §3.1 echo rule for this kind reduces to
  the three fields above; the snapshot Reversible already
  carries the full row, so adding a `created_at_ms` /
  `updated_at_ms` pair would change the wire shape of the
  audit-log row without operator benefit. Deferred until
  `tenant.list` actually surfaces "last touched" to an operator.

Two migrations in the source:
1. `0001_create_rubix_tenants.sql` — schema above.
2. `0002_seed_system_tenant.sql` — `INSERT ... ON CONFLICT DO
   NOTHING` for the bundled `("system", "System", "en")` row
   the in-memory `InMemoryTenantStore::seeded(...)` boot path
   has been installing. Without this, a fresh Pg-backed boot
   would surface an empty tenant directory and the bundled
   SDUI pages (whose `tenant_id` is `BUNDLED_TENANT`) would
   resolve against nothing.

### Trait placement — moved to `rubix-spi::tenant`

Same decision as the audit-policy slice and same reasoning: the
workspace `Cargo.toml` documents `rubix-store-postgres` as a
sibling of `rubix-tools` rooted in `rubix-spi`. Hoisting the
trait + row keeps the dep arrow correct and pulls zero `tools`
compile graph into the thin DB layer.

The move shipped as:

- `rubix-spi/src/tenant/{mod.rs, store.rs}` — `TenantRow` (now
  `PartialEq, Eq` so the test can assert `==`), `TenantStore`
  trait, `TENANT_KIND`.
- `rubix-tools/src/tenant/store.rs` — `pub use` re-exports from
  spi at the top so every verb's `use crate::tenant::store::...`
  keeps compiling. Kept locally: `InMemoryTenantStore` (with
  its `seeded` / `insert` convenience constructors),
  `TenantReversible`, `parse_row`, and the four existing tests.

### Conflict mapping — Postgres error -> `Error::Conflict`

The in-memory `create` returns two different
`Error::Conflict { message }` strings ("tenant with id X already
exists" vs "tenant with name Y already exists"). Both are
operator-visible and the verb body relays them verbatim. The Pg
impl inspects `sqlx::DatabaseError::code()` for `23505`
(unique_violation) and `constraint()` to discriminate
`*_pkey` (id collision) vs the unique-name index, reproducing
the in-memory messages exactly. The new test asserts both
messages name the right offending value.

### Behavior parity — `delete` returns `NotFound`

The trait contract says `delete` on a missing id returns
`Error::NotFound`. The audit-policy `delete` was idempotent
(different contract); the tenant Pg impl checks
`rows_affected() == 0` after the DELETE and synthesises the
error to match the in-memory impl. Tested.

## Wiring

- `rubix-spi/src/lib.rs` — `pub mod tenant;`.
- `rubix-store-postgres/src/lib.rs` — `pub mod tenants;`,
  `pub use tenants::PgRubixTenantStore;`, plus the new
  `RUBIX_TENANTS_MIGRATOR` / `RUBIX_TENANTS_MIGRATION_SOURCE`
  pair (same shape as `DASHBOARDS_DEFINITIONS_*`).
- `rubix-agent/src/registry.rs` — `match pg_pool` selection;
  the in-memory `seeded(...)` branch keeps its
  `BUNDLED_TENANT` / `"System"` row so the two boot paths
  surface identical directories to the verbs.
- `rubix-agent/src/boot/migrations.rs` — appended
  `RUBIX_TENANTS_MIGRATION_SOURCE` to the boot chain after the
  dashboards/scheduled-flows sources. Ordering is independent
  of the existing sources (no FKs).

No verb code, no DTO, no descriptor changes. The slice is pure
substrate.

## Tests

- `rubix-tools::tenant::store::tests` (in-tree) — four existing
  tests still pass against the in-memory store after the trait
  moved; the `pub use` shim guarantees identical type paths.
  Added `Eq` derive to `TenantRow` (was `PartialEq` only) so
  the new Pg test can assert structural equality without a
  helper.
- `rubix-agent/tests/tenant_pg_test.rs` (new) — six scenarios
  behind `#[ignore]` (testcontainers Postgres):
  1. Seed row visible: `list` returns `[system]`.
  2. `create` round-trips a fresh row; `get` echoes byte-exact.
  3. Duplicate id rejected with id-bearing `Conflict` message
     (PRIMARY KEY path).
  4. Duplicate name rejected with name-bearing `Conflict`
     message (UNIQUE constraint path).
  5. `put` bypasses uniqueness and restores a snapshot
     verbatim (undo path).
  6. `delete` removes the row; second `delete` returns
     `NotFound`.

## Gates

- `cargo build --workspace` clean.
- `cargo test -p rubix-tools --lib` -> 276 / 276 green
  (existing tenant tests still pass against the relocated
  trait via the `pub use` shim).
- `cargo clippy -p rubix-tools -p rubix-spi -p rubix-agent
  -p rubix-store-postgres --lib --tests` clean for my files
  (pre-existing warnings in `cleaner/`, `chat_stream.rs`,
  `admin/source.rs`, `warehouse_write.rs` are off-slice).
- `cargo test -p rubix-agent --test undo_dispatch_test
  --test goal_2_user_admin_test --test admin_registry_test`
  -> 3 / 3, 2 / 2, 9 / 9 green.
- `cargo test -p rubix-agent --test tenant_pg_test --no-run`
  compiles (no Docker in sandbox to run the ignored cases).

## Off-limits respected

Same admin zone as the previous slice — no edits to
`admin/`, `boot/auth.rs`, `tail_listen.rs`. `boot/migrations.rs`
is on-limits and the only boot file touched.

## Follow-ups

- `PgUserAdminStore` next. The user table is bigger
  (`prefs_json`, `tenant_id` FK candidate, `disabled_at_ms`)
  but follows the same pattern. Will need a decision on whether
  the `tenant_id` column carries an FK to `rubix_tenants` now
  that the table exists, or stays loose-text for symmetry with
  the in-memory impl. Recommend FK with `ON DELETE RESTRICT`
  to honour the tenant-delete refuse-if-referenced check at the
  DB layer too.
- The `rubix_tenants` table has no `created_at_ms`/`updated_at_ms`.
  If a future operator wants "tenant last touched" surfaced in
  the list verb, add the columns + a non-breaking
  `TenantRow.{created_at_ms, updated_at_ms}: Option<i64>` (default
  `None` for older rows) and update the snapshot path then.
