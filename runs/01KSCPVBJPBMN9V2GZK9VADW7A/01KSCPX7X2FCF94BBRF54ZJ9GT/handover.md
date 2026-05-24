## Done

- Created `starter-extensions/crates/starter-ext-store-pg/` with Cargo.toml (deps: starter-ext-spi, starter-ext-server, sqlx postgres+chrono, async-trait, serde, thiserror).
- `src/lib.rs` barrel re-exporting `PgEnablementStore`.
- `src/store.rs` (~135 lines) — `PgEnablementStore { pool: PgPool }` impl `EnablementStore` (get + set via UPSERT); inherent helpers `set_as(actor, id, state)` (audited writes) and `list_all() -> Vec<(ExtensionId, EnablementState)>` (ordered by extension_id).
- `src/migrations/0001_extensions_enablement.sql` — `extensions_enablement(extension_id PK, state CHECK enabled|disabled, updated_at TIMESTAMPTZ default NOW(), updated_by TEXT)`.
- Added crate to `starter-extensions/Cargo.toml` `[workspace.members]`.
- `cargo build -p starter-ext-store-pg` green; `cargo build -p starter-ext-server` still green.
- Committed as `d6a1827`.

## Next

- Stage 2 (per SCOPE Phase A continuation): testcontainers-backed `tests/store_test.rs` for `PgEnablementStore` roundtrip + UPSERT + list ordering, plus SCOPE.md "Decisions made — enable/disable persistence model" one-line addition naming starter-ext-store-pg.
- Then Phase B (`rubix/extensions/` workspace + example builds), Phase C (rubix-agent boot wiring), Phase D (install + test-ui-5), Phase E (tests/docs/PR).

## What you need to know

- `starter_ext_server::StoreError` was previously only reachable through the private `store` module path; lib.rs now re-exports it alongside `EnablementState`/`EnablementStore`/`InMemoryEnablementStore`. This is additive, not a breaking change; verify nothing downstream collides.
- The sqlx `query_as` calls need explicit turbofish (`::<_, (String, String)>`) for tuple inference under sqlx 0.8 — leave as-is.
- starter-extensions is its own workspace and does not have sqlx in `[workspace.dependencies]`; the new Cargo.toml declares sqlx directly with `version = "0.8"` and the same feature pinning as the parent workspace (runtime-tokio, macros, postgres, chrono).
- Migration is shipped as a plain `.sql` file under `src/migrations/`; callers run it via sqlx's standard `Migrator` against this dir (rubix-agent will wire that in Phase C).

## Open questions

- (none)
