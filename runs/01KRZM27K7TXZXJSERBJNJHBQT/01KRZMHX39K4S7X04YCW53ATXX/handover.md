## Done

- Added `LinkedProvidersLookup` trait + `NoLinkedProviders` default in `crates/starter-auth-users/src/linked_providers.rs`, exported from the crate root.
- `AuthState` now carries `Arc<dyn LinkedProvidersLookup>` with `NoLinkedProviders` default; new `with_linked_providers` builder.
- `UserRecord.password_hash` is `Option<String>`; `UserStore::create` takes `Option<&str>`; sqlite impl updated (nullable read + bind).
- `POST /auth/login` on `NULL` hash returns HTTP 400 `{ error: "password_not_set", providers: [...] }`. Added `PasswordNotSetResponse` schema, openapi snapshot refreshed.
- `create_admin` call site updated to pass `Some(&hash)`.
- Migration `0002_users_password_optional.sql` shipped in `crates/starter-auth-oauth/migrations/starter_auth_oauth_{sqlite,postgres}/` (NOT in starter-auth-users). sqlite path is the 12-step rebuild with `PRAGMA foreign_key_check`; postgres is `ALTER COLUMN DROP NOT NULL`.
- `starter-store-sqlite::migrate::runner` now honours sqlx's `-- no-transaction` marker so the rebuild migration can disable FKs.
- Tests cover NULL-hash round-trip and the `password_not_set` HTTP shape with stub + default lookups.
- `cargo check --workspace --all-features --all-targets`, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings` all pass.
- Committed as `f03cea2` on `codeless/starter-auth-oauth`.

## Next

- Stage 4 (Phase 1b): scaffold the `starter-auth-oauth` crate — `OAuthProvider` / `ProviderIdentity` / `OAuthFlowState` / `OAuthStateStore` types, `MemoryStateStore`, `IdentityStore` sqlite impl + `0001_oauth_identities.sql`, `OAuthLinkedProviders` impl, config wiring through `starter-secrets-*`.

## What you need to know

- The OAuth crate directory `crates/starter-auth-oauth/` currently exists but contains only `migrations/`. Stage 4 must add `Cargo.toml`, `src/`, and register the crate in the workspace `Cargo.toml` members list (not yet added — workspace still builds because the directory has no `Cargo.toml`).
- `starter-auth-users` tests now apply the OAuth migration inline via `sqlx::migrate!("../starter-auth-oauth/migrations/starter_auth_oauth_sqlite")`. Stage 4's `0001_oauth_identities.sql` will sit in the same directory and run in the same sequence.
- The sqlx `-- no-transaction` marker must be on the FIRST line of any migration that needs FK pragmas to take effect.
- The custom `starter-store-sqlite` runner now branches on `migration.no_tx`; if a future migration uses the marker, the per-statement error surface is different (no automatic rollback). Keep that in mind for any future destructive migration.
- `cargo tree -p starter-auth-users` still must not list `starter-auth-oauth` — the seam stays one-directional via the `LinkedProvidersLookup` trait.

## Open questions

- (none)
