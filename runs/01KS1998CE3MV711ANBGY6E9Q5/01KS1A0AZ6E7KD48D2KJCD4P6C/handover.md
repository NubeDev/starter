## Done

- Added `sqlite` cargo feature (default-off, per R5) wiring `sqlx/sqlite,migrate,runtime-tokio` and dev-deps (tokio, sqlx with sqlite/migrate).
- Added migration `crates/starter-prefs/migrations/0001_starter_prefs.sql` with `starter_prefs_org` + `starter_prefs_user` per SCOPE "Preferences model"; updated_at INTEGER (UTC epoch ms).
- Rewrote `crates/starter-prefs/src/store.rs`: `PrefsStore` async_trait (get/upsert × user/org) always compiled; sqlx-based `SqlitePrefsStore` gated on `sqlite`, embedded `MIGRATIONS` via `sqlx::migrate!`. Column codecs preserve `UnitPref::Auto` / `StringPref::Auto` sentinels and round-trip every enum verbatim.
- Added integration tests under `crates/starter-prefs/tests/sqlite_store.rs` against `sqlite::memory:` covering round-trip preservation (user + org), NULL → None, upsert overwrite, missing row → None, and the N=7 multi-org keyed-row contract. All 6 tests green; resolver unit tests still green.
- Commit `51f4e07` on branch `codeless/starter-prefs-i18n`.

## Next

- Stage 6 picks up from a clean baseline; per the job plan stages 6+ add starter-client-rs methods, starter-cli `prefs` subcommand, and the REST routes/middleware behind the `routes` feature.

## What you need to know

- `PrefsStore` is `Send + Sync` and uses `async_trait` (matches `ThemeStore` precedent).
- `upsert_*` writes a full row (None → NULL); patch-merge semantics were deliberately deferred — the SCOPE roundtrip contract treats NULL as "inherit", not "leave unchanged on update".
- `MIGRATIONS` is `pub static` (only present under `sqlite`) so consumers can apply it against their own pool if not using `SqlitePrefsStore::migrate`.
- `updated_at` is stamped server-side inside `upsert_*` via `SystemTime::now()`; it's not part of the row types (resolver never reads it).
- Enum DB encoding uses `serde_json::to_value` → JSON string; works because every preference enum in `starter-spi` serializes to a bare string. If a future variant ever serializes non-string, `enum_to_db` will panic at runtime — flagged as a future-proofing risk.

## Open questions

- (none)
