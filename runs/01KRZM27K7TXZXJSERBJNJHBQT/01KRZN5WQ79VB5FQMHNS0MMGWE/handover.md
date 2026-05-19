## Done

- Registered `starter-auth-oauth` in the workspace (`Cargo.toml` members + workspace.dependencies).
- `crates/starter-auth-oauth/Cargo.toml` depends on `starter-auth-users`, `starter-spi`, `oauth2 = "5"` (the line that targets `reqwest 0.12`), `reqwest`; `sqlx` + `starter-store-sqlite` behind the `sqlite` feature.
- `src/lib.rs` exposes the scaffold types only — module slots for `providers/`, `routes/`, `session_bridge` are reserved for later stages.
- `src/provider.rs`: `OAuthProvider` trait + `ProviderIdentity { provider_sub, email, email_verified, display_name }` + `ProviderError` that carries no access-token material.
- `src/state_store/{mod,memory}.rs`: `OAuthFlowState`, `OAuthStateStore` trait, `MemoryStateStore` (HashMap<state, Flow> behind `std::sync::Mutex`, 10-minute `STATE_TTL`, opportunistic sweep on every `take`).
- `src/identity_store/{mod,sqlite}.rs`: `IdentityStore` trait + `SqliteIdentityStore` (feature `sqlite`) with `find`/`insert`/`delete`/`list_for_user`; `linked_at` round-trips through the same `TEXT` shape sqlite's `CURRENT_TIMESTAMP` uses.
- `src/linked_providers.rs`: `OAuthLinkedProviders` implements `starter_auth_users::LinkedProvidersLookup` by querying `list_for_user` and de-duplicating to provider ids in `linked_at`-ascending order.
- `src/config.rs`: `OAuthConfig::load(Option<&dyn SecretStore>)` resolves `OAUTH_BASE_URL`, `OAUTH_STATE_STORE`, `OAUTH_SIGNUP_ENABLED`, `OAUTH_SIGNUP_DEFAULT_ROLE`, and per-provider `OAUTH_<P>_CLIENT_ID/_CLIENT_SECRET` through the secret store (preferred) with env-var fallback. Presence of both client_id+secret enables the provider; partial credentials warn and disable. `client_secret` is wrapped in `Secret`.
- Migrations `0001_oauth_identities.sql` (sqlite + postgres): composite PK `(provider, provider_sub)`, FK to `starter_auth_users_users(id) ON DELETE CASCADE`, `idx_oauth_identities_user` on `user_id`.
- `tests/identity_store.rs`: integration test runs users + oauth migrations in order and exercises insert/find/delete/list + `OAuthLinkedProviders` ordering and de-duplication.
- `cargo check --workspace --all-features --all-targets`, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-features --all-targets -- -D warnings` all pass. `cargo tree -p starter-auth-users` still does not list `starter-auth-oauth`.
- Committed as `c4c005a` on `codeless/starter-auth-oauth`.

## Next

- Stage 5 (Phase 1c): implement `providers/github.rs` with compile-time scopes `read:user user:email` + endpoint constants; build `oauth_router` mounting `start` + `callback` under a consumer prefix; wire the seven-branch callback flow against the existing `IdentityStore` / `MemoryStateStore` / `OAuthLinkedProviders` and the `starter-auth-users` `SessionStore`. Each branch needs its own test, plus the access-token-never-persists static + runtime guards.

## What you need to know

- The migration loader orders by numeric prefix, so `0001_oauth_identities.sql` runs before `0002_users_password_optional.sql`. The 0002 rebuild renames the users table by name; the FK in 0001 reconnects automatically. `PRAGMA foreign_key_check` inside 0002 is the guard.
- Provider id list is currently hard-coded in `config.rs` as `KNOWN_PROVIDERS = &["github", "google"]`. Adding a third provider is one entry there + one file in `providers/`.
- `MemoryStateStore::take` evicts expired entries on every call (not just the one being read); a future `SqliteStateStore`/`PostgresStateStore` should match that contract.
- `SqliteIdentityStore` re-exported at the crate root behind `cfg(feature = "sqlite")`. Tests need `--features sqlite` (or rely on `--all-features`).
- `oauth2 = "5"` is declared but unused in this stage — Stage 5 / Phase 1c will start consuming its PKCE + token-exchange types.
- `linked_at` is bound on `insert` as `format("%Y-%m-%d %H:%M:%S")` and parsed back through three lenient `NaiveDateTime` patterns; if a future migration switches the column type to `INTEGER` epoch, `parse_ts` needs to change.
- One-way dep arrow holds (`cargo tree -p starter-auth-users` is clean).

## Open questions

- (none)
