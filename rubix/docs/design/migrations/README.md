# MIGRATIONS — starter + rubix migration order

> **Phase 2a entry gate.** Resolve before code lands.

## The problem

Rubix consumes many `starter-*` crates that ship their own schema
migrations (`starter-auth-users`, `starter-authz`, `starter-store-
postgres` namespaces, etc.). Rubix-specific tables (e.g. the
dashboard SDUI page store, if not living in `starter-sdui-routes`)
also need migrations.

## The boot order

```
1. Open the Postgres pool (starter-store-postgres).
2. Run every starter crate's migration source it carries
   (`source = "starter-auth-users"`, `source = "starter-authz"`, ...).
3. Run rubix's own migration source (`source = "rubix"`).
4. Run any extension-contributed migration sources.
5. Then — and only then — open the listening sockets.
```

If step 2 or 3 fails, the binary exits non-zero. **Never** start
serving on a partially-migrated DB.

## Where rubix migrations live

`rubix-agent/migrations/` — one directory per migration version,
shipping `up.sql` + `down.sql`. The crate registers them with the
namespaced migration runner from `starter-store-postgres` under
`source = "rubix"`.

## Cross-crate dependencies

A rubix migration **may** reference a starter table by name, but
**may not** add a foreign key to a starter table. Reason: starter's
own ADRs forbid cross-tree FKs (the version skew breaks ordering).
Instead, rubix joins logically at query time.

## Testing

Every migration ships with:
1. An `up.sql` that applies cleanly on the previous version's DB.
2. A `down.sql` that reverses it.
3. An integration test in the matching crate using the
   testcontainers pattern from `starter-store-postgres`.

## Extension migrations

Phase 5 — `starter-ext-flow` (upstream) loads extension-contributed
migrations under their own `source` namespace. Rubix does not own
this; the host runner does.

## Phase 2a exit signal

`cargo run -p rubix-agent` on a fresh Postgres applies every
migration (starter + rubix) in order, exits 0 if all run cleanly,
exits non-zero with a clear error otherwise. No half-state.
