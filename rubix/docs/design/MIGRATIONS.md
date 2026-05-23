# MIGRATIONS — the platform/product split for SQL

> Source: `rubix/SCOPE.md` §"Migrations across the platform/product
> split", "Decisions made" (migrations-order bullet), R4 (layer
> arrow), R5 (contracts hub), ADR-001 (no SQLite — Postgres only).
> Cross-refs: `AUTH.md` (the `starter_auth_*` sources), `TESTS.md`
> (testcontainers run migrations before each test).

This doc defines who owns which migrations, what order they run at
boot, the rule against cross-tree foreign keys, and the forward-only
rollback convention.

## The rule

> **`starter-store-*` owns the platform schemas. `rubix/agent/crates/
> data-postgres` owns the rubix-specific schemas. Migrations run
> `starter_*` first, `rubix` second. Forward-only. A `rubix`
> migration MAY reference a `starter_*` table; a `starter_*`
> migration MAY NEVER reference a `rubix` table.**

The asymmetry is load-bearing. `starter` ships independently of any
consumer; it cannot know about `rubix` tables. `rubix` knows about
`starter` (it consumes it) and is allowed to FK into the platform
tables. Reversing this direction would couple the platform to a
product and ADR-001's "platform ships independently" guarantee
would collapse.

## Namespaced migration runner

`starter-store-postgres` ships a **namespaced migration runner**:
one `_sqlx_migrations_<source>` table per source string. There are
no version-number collisions between sources; each component
manages its own forward-only sequence.

Sources currently in use:

| Source | Owner | What it owns |
|---|---|---|
| `starter` | `starter-server` core | Request-id table, server bookkeeping (if any). |
| `starter_auth_users` | `starter-auth-users` | Users, sessions, tokens, tenants, teams (Phase 7). |
| `starter_auth_oauth` | `starter-auth-oauth` | OAuth identities + IdP links. |
| `starter_authz` | `starter-authz` | Policy tables, decision audit log. |
| `starter_prefs` | `starter-prefs` | Per-user / per-tenant preferences. |
| `rubix` | `rubix/agent/crates/data-postgres` | Every rubix-specific table: devices, points, schedules, alarms, history dimensions, dashboards, … |

The `source` string is **stable per component**. Adding a new
component adds a new source; never rename an existing one. Renaming
would force every existing deployment to re-run from scratch under
the new name and likely fail uniqueness checks.

## Where the migration files live

```
starter/crates/starter-store-postgres/src/migrations/runner.rs
                                                  ↑
                                            (the namespaced runner)

starter/crates/starter-auth-users/migrations/<source=starter_auth_users>/
starter/crates/starter-authz/migrations/<source=starter_authz>/
starter/crates/starter-prefs/migrations/<source=starter_prefs>/

rubix/agent/crates/data-postgres/migrations/<source=rubix>/
    20260101_000000_devices.sql
    20260101_000001_points.sql
    20260201_000000_schedules.sql
    …
```

The naming convention is `YYYYMMDD_HHMMSS_<concept>.sql`. The
`<concept>` segment names the *what* (`devices`, `points`,
`schedules`) — never the *shape* (`utils`, `misc`, `common`); R1's
no-name-only-by-shape rule applies to migration files too.

## Boot-time ordering

At agent startup, `apps/agent/src/main.rs` orchestrates migrations:

```text
1. Open the Postgres pool (data-postgres).
2. Run starter_* sources, in dependency order:
   2a. starter
   2b. starter_auth_users
   2c. starter_auth_oauth
   2d. starter_authz       (depends on tenants/teams from auth_users)
   2e. starter_prefs
3. Run rubix source.
4. Begin serving requests.
```

The agent **does not start serving requests** until every source
applied cleanly. A migration failure is a hard exit (`std::process::
exit(1)`) with a structured `tracing::error!` log. The orchestration
itself is a thin wrapper in `apps/agent`; the heavy lifting
(per-source idempotency, advisory locks, etc.) is the
`starter-store-postgres` namespaced runner's job.

CI's "agent boots cleanly with empty schema" test (Phase 0 exit gate)
exercises this orchestration end to end on an empty testcontainer
Postgres.

## The no-cross-tree-FK rule

> A `rubix` migration MAY add a column referencing a `starter_*`
> table (e.g. `devices.tenant_id REFERENCES
> starter_auth_users_tenants(id)`).
>
> A `starter_*` migration MAY NEVER add a column referencing a
> `rubix` table.

This is enforced at CI time by parsing the migration text. The
parser:

1. Lists every `CREATE TABLE` and every `ALTER TABLE … ADD
   CONSTRAINT … FOREIGN KEY REFERENCES <table>` in each source.
2. For each `REFERENCES <table>` in a `starter_*` source, asserts
   the referenced table name does **not** appear in any rubix-owned
   `CREATE TABLE`. (The rubix table-name set is computed by walking
   `rubix/agent/crates/data-postgres/migrations/`.)
3. Fails the build with a precise message naming the offending
   migration file and table.

The parser lives next to the `mani run lint` task. False positives
(a `starter_*` table name that happens to match a rubix table by
coincidence) get explicit-allowlist comments in the lint config;
new rubix table names that collide must rename to avoid the
collision.

## Rollback rule — forward-only

> **Rollbacks happen by adding a new migration that reverses the
> prior one — never by editing or deleting a checked-in file.**

A checked-in migration is permanent. If it was wrong:

1. Land a **new** migration that performs the reversing operation
   (`DROP COLUMN`, `DROP CONSTRAINT`, `UPDATE … SET old = new`,
   etc.).
2. Land a **further** migration if needed to roll forward to the
   correct shape.
3. Coordinate the consumer code change in the same PR.

Why: editing or deleting an applied migration breaks every
deployment that already ran it. The `_sqlx_migrations_<source>`
table records the checksum; a mutated migration file fails the
checksum check at the next boot and the agent refuses to start.

If a starter migration introduces a constraint that a rubix
migration relies on, removing the starter migration breaks rubix.
The fix is the **coordinated bump** (R10): add a new migration on
both sides, in one PR, with a coordinated version bump on the
crates.

## Inter-source dependency rules

A rubix migration may reference a `starter_*` table. The rules:

- **Same boot, same database.** Both sources run before the agent
  serves; ordering above guarantees the FK target exists when the
  rubix migration runs.
- **Lookups, not lifecycles.** A FK from `devices.tenant_id` →
  `starter_auth_users_tenants(id)` is fine because the rubix code
  does not delete tenants. Cascading deletes that span the boundary
  are forbidden — `ON DELETE CASCADE` from a starter table into a
  rubix table would let `starter_auth_users` reach across the
  boundary at run-time, and the asymmetry inverts.
- **Use `ON DELETE RESTRICT` or `SET NULL`.** The rubix domain
  decides what to do when a referenced platform row goes away;
  never let the platform cascade.

## Forward-only manifest migrations (cross-link)

`KIND-MANIFEST.md` covers manifest version bumps. When a kind's
`version: u32` bumps and existing persisted slot data needs
transforming, the **manifest migration** is a separate concept
from the SQL migration — it runs as part of kind registration in
`apps/agent` (kinds-registry walks the persisted graph, applies the
manifest migration per kind, then begins normal serving). The SQL
migration shape (DDL) and the manifest migration shape (data
transforms) are decoupled; a kind bump may need both, one, or
neither.

## ClickHouse — separate ground

`starter-store-clickhouse` has its own migration runner. The same
rules apply: a `rubix-clickhouse` source for warehouse-side DDL
(L1 raw / L2 curated / L3 marts), forward-only, with the
warehouse SCOPE's tag-driven mart definitions on top. The
warehouse is read-side and eventually consistent vs. Postgres; the
ingest path is the seam (no read-through). See SCOPE.md §"Storage
roles — Postgres vs ClickHouse" and the warehouse SCOPE under
`DOCS/Warehouse/SCOPE.md`.

The boot ordering for ClickHouse is independent of Postgres
ordering: ClickHouse migrations run after Postgres in a sequential
boot but the two failures are surfaced separately so an operator
can disambiguate.

## ADR-001 — no SQLite

ADR-001 in `starter` ("Postgres only — no SQLite anywhere") extends
to the whole rubix stack. No rubix migration ever runs against
SQLite. No `data-sqlite-*` crate exists in `rubix/agent/crates/`.
The "Postgres-only" smoke test (OVERVIEW.md) catches any sneak-in:
`cargo tree` from `apps/agent` shows zero resolution to any
`*-sqlite-*` crate.

## Authoring a rubix migration — checklist

1. **Name the file** by concept: `YYYYMMDD_HHMMSS_<concept>.sql`.
   Concept singular, lowercase, snake_case. No `utils`, no
   `common`, no `misc`.
2. **One responsibility per file.** A migration that adds a new
   table is one file; a migration that adds a column to that table
   weeks later is another file. R1's "one responsibility" rule
   applies to SQL too.
3. **No `IF NOT EXISTS` papering.** A migration runs exactly once;
   `IF NOT EXISTS` masks "I already ran this in a hand-edited
   deployment" bugs. The runner guarantees once-only application
   via the per-source migrations table.
4. **No `DROP TABLE` of a previously-existing rubix table** without
   a follow-up confirmation. Use `ALTER TABLE … RENAME TO
   <table>_archived_YYYYMMDD` if you mean "stop using"; drop in a
   later migration after the archive period passed.
5. **Use `CREATE TYPE` for enums** rather than `VARCHAR + CHECK`.
   Type changes are then a `CREATE TYPE / ALTER TABLE … TYPE`
   sequence rather than a constraint-rewriting round.
6. **FKs to `starter_*` are typed as `UUID` (or matching native
   type)**, never as `VARCHAR(36)`. Type drift across the boundary
   creates silent join-cost bugs.
7. **Add the corresponding test** under
   `rubix/agent/crates/data-postgres/tests/` (`TESTS.md` —
   `#[ignore = "needs-docker"]`, runs against a testcontainer).
   The test asserts the migration applies cleanly to an empty
   schema and the resulting shape passes a smoke insert + select.
8. **Run `mani run lint`** — the cross-tree-FK parser runs as part
   of lint. A starter-to-rubix FK fails the build with a precise
   error message.

## Common pitfalls

- **"I'll just edit the migration that landed yesterday."** No.
  Every deployment that pulled main and booted has applied the
  checksummed version. Land a new migration.
- **`ON DELETE CASCADE` from a starter table into a rubix table.**
  Inverts the platform/product asymmetry; the parser catches it,
  but if it ever lands, the platform can now reach into rubix at
  run-time and ADR-001's independence is dead.
- **Renaming a `source` string.** Forces every existing deployment
  to re-run the whole sequence under the new name. The runner sees
  zero applied migrations and tries to apply them all again,
  failing on the first table that already exists. Don't.
- **A migration that references a kind manifest field by name.**
  Manifests bump independently (`KIND-MANIFEST.md`); a migration
  that depends on a specific slot name freezes the manifest
  version. Use opaque columns (`slot_value JSONB`) keyed off the
  manifest at read time; the kind code is the authority for shape.
- **Running migrations from a domain crate's test harness directly,
  bypassing the namespaced runner.** Tests must go through
  `starter-store-postgres::testing::with_database()` so the
  per-source isolation is exercised; ad-hoc `sqlx::migrate!()`
  calls in tests miss the cross-source ordering bugs.

## Phase 1 entry expectation

For Phase 1, the rubix source needs three tables at minimum:

- `rubix_devices` — device dimension, FK `tenant_id` →
  `starter_auth_users_tenants(id)`, `placement_path`,
  `created_at`, `kind`, `version` (the manifest version this row was
  created against).
- `rubix_points` — point dimension, FK `device_id` →
  `rubix_devices(id)`, `slot_key`, `kind`, `version`.
- `rubix_slot_values` — last-known slot value per `(node_id,
  slot_key)`, the persistent backing for the graph store.

Each table lands as its own migration file. Each has an integration
test under `data-postgres/tests/` exercising apply + smoke insert.
None of these tables is referenced from any `starter_*` migration —
the parser will confirm.
