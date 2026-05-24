## Done

- Added `tests/store_test.rs` for `starter-ext-store-pg` exercising get/set roundtrip, UPSERT idempotence, `list_all` ordering, and `updated_by` audit (`system` vs `set_as`).
- Added `testcontainers` feature flag (off by default) and dev-deps on `starter-store-postgres/testing` (path) + `tokio` (macros + rt-multi-thread).
- `cargo test -p starter-ext-store-pg --features testcontainers` runs and passes (1.86s, container-backed).
- Appended one-line decision under "enable/disable persistence model" in `DOCS/extensions/scope/SCOPE.md` naming `starter-ext-store-pg` as the default DB-backed impl.
- Committed as `6d893b3` on `codeless/rubix-extensions-wire`.

## Next

- Stage 3 of the rubix-extensions-wire job (next phase, picked up in fresh session).

## What you need to know

- Migration applied via `sqlx::query(include_str!(...))` rather than a `Migrator` — keeps test self-contained and asserts the raw SQL works.
- `starter-store-postgres` is in the parent workspace; pulled as a direct path dep (`../../../crates/starter-store-postgres`) in dev-deps rather than via `workspace.dependencies` since the testcontainers harness is dev-only.
- `Pool::sqlx()` returns the underlying `PgPool`; cloned into `PgEnablementStore::new`.

## Open questions

- (none)
