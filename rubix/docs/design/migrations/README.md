# MIGRATIONS — starter + rubix migration order

## The problem

Rubix consumes many `starter-*` crates that ship their own schema
migrations (`starter-auth-users`, `starter-authz`, the
`starter-store-postgres` namespaces, etc.). Rubix-specific tables
(e.g. the dashboard SDUI page store, if not living in
`starter-sdui-routes`) also need migrations. Without a defined
order, two crates racing on the same schema produces flaky boots.

## The boot order

```
1. Open the Postgres pool (starter-store-postgres).
2. Run every starter crate's migration source it carries
   (source = "starter-auth-users", source = "starter-authz", ...).
3. Run rubix's own migration source (source = "rubix").
4. Run any extension-contributed migration sources.
5. Then — and only then — open the listening sockets.
```

If step 2 or 3 fails, the binary exits non-zero. The agent never
serves on a partially-migrated DB.

## Where rubix migrations live

`rubix-agent/migrations/` — one directory per migration version,
each shipping `up.sql` + `down.sql`. The crate registers them
with the namespaced migration runner from `starter-store-postgres`
under `source = "rubix"`.

## Cross-crate dependencies

A rubix migration **may** reference a starter table by name, but
**may not** add a foreign key to a starter table. Reason:
starter's ADRs forbid cross-tree FKs because version skew between
the rubix migration and the starter migration breaks the strict
boot order — a rubix FK pointing at a starter table the local
starter version has not yet created (or has renamed) fails the
whole boot. Rubix joins logically at query time instead.

The same rule applies in reverse: starter crates never FK into
rubix tables. They do not know rubix exists.

## Testing

Every migration ships with:

1. An `up.sql` that applies cleanly on the previous version's DB.
2. A `down.sql` that reverses it.
3. An integration test in the matching crate using the
   testcontainers pattern from `starter-store-postgres`, marked
   `#[ignore]` so it only runs in the integration job.

The pattern: spin up a Postgres container, run every prior
migration, apply `up.sql`, assert schema, apply `down.sql`,
assert reverted.

## Extension migrations

`starter-ext-flow` (Phase 5) loads extension-contributed
migrations under their own `source` namespace. Rubix does not own
extension migration ordering; the host runner does. An extension
that needs to reference a rubix table follows the same
no-cross-tree-FK rule.

## Verification

`cargo run -p rubix-agent` on a fresh Postgres applies every
migration (starter + rubix) in order, exits 0 if all run
cleanly, exits non-zero with a clear error otherwise. No
half-state. The migration phase happens before the listening
socket opens, so a failed migration never produces a half-serving
binary.

## What rubix does NOT do

- **No "skip-on-error" mode.** A failed migration is a hard
  boot failure.
- **No "auto-rollback on partial failure".** If `up.sql` fails
  mid-statement, operator intervention is required. Migration
  authors keep statements in a single transaction where possible.
- **No bypass for development.** Dev mode runs the same boot
  order; the only difference is the DSN.
