## Done

- Added `postgres` feature to `crates/starter-prefs/Cargo.toml` (enables `sqlx/postgres`, `sqlx/migrate`, `sqlx/runtime-tokio`); consumers can activate `sqlite`, `postgres`, or both independently.
- Added `starter-store-postgres` (feature = `testing`) to dev-dependencies for the testcontainers helper.
- Created `crates/starter-prefs/migrations/postgres/0001_starter_prefs.sql` — Postgres dialect rewrite; sole difference from SQLite migration is `updated_at BIGINT` (explicit 64-bit) vs SQLite's `INTEGER` affinity; no rowid or affinity quirks to handle.
- Extended `crates/starter-prefs/src/store.rs` with: `PG_MIGRATIONS` static (`sqlx::migrate!("./migrations/postgres")`), `postgres_impl` module behind `#[cfg(feature = "postgres")]`, and `pub use postgres_impl::PgPrefsStore`. Shared codec helpers (`enum_to_db`, `unit_pref_from_db`, etc.) now use `#[cfg_attr(not(any(feature = "sqlite", feature = "postgres")), allow(dead_code))]`.
- Created `crates/starter-prefs/tests/postgres_store.rs` — 6 tests ported from `sqlite_store.rs`, all `#[ignore = "requires docker"]`, gated `#![cfg(feature = "postgres")]`.
- Confirmed: `cargo test -p starter-prefs --features sqlite` → 6/6 pass; `cargo test -p starter-prefs --features postgres -- --ignored` → 6/6 pass.
- Committed as "slice C — starter-prefs Postgres backend" (77972b2).

## Next

- Stage 5 (slice D): port `starter-store-sqlite::flow` to `starter-store-postgres` (preserving the `starter-flow-spi` trait surface).
- After slice D: REVIEW gate 2.
- Stages 6–7: rewire `examples/flow-agent` to Postgres-only, switch its tests to testcontainers, delete SQLite migrations from the example.

## What you need to know

- `PgPrefsStore::new(pool: sqlx::PgPool)` mirrors `SqlitePrefsStore::new(pool: SqlitePool)` exactly.
- `PG_MIGRATIONS` is exported from `starter_prefs::store` (only when `feature = "postgres"` is active); `MIGRATIONS` (SQLite) is unchanged.
- The Postgres migration path is `./migrations/postgres/` relative to the crate root — separate from the SQLite migration dir `./migrations/`.
- `examples/flow-agent` has NOT been touched; this stage is library-only per the spec.
- No SQLite code was removed from any crate.

## Open questions

- (none)
