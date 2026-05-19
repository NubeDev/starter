## Done

- Added `SqliteStateStore` + `PostgresStateStore` behind `sqlite` / new `postgres` cargo features; postgres feature also pulls `sqlx/chrono`. Trait-identical `put` / `take` semantics (memory ≡ sqlite ≡ postgres).
- Migration `0003_oauth_state.sql` per backend creates `starter_auth_oauth_state` (state PK + 5 cols) + `idx_oauth_state_created_at` for the opportunistic TTL sweep inside `take`.
- `ProviderCredentials.role_domain_map: HashMap<String, Role>` parsed from `OAUTH_<PROVIDER>_ROLE_DOMAIN_MAP=acme.com=Writer,...` (lowercased on parse, strict parser surfaces config errors at startup).
- `OAuthRoutesState.role_domain_maps` + `resolve_signup_role` helper in `routes/callback.rs` — verified-email domain → role at signup, default-role on any miss. Only runs after the `email_verified` guard.
- Re-exports added to `lib.rs`; `MemoryEverything` testing factory initialises `role_domain_maps` empty.
- Smoke tests: `tests/state_store_cross_instance.rs` (sqlite handoff between two `OAuthRoutesState`s succeeds; memory state store fails fast with `sign_in_failed` BAD_REQUEST, no provider IO) and `tests/domain_map_role.rs` (acme.com=Writer; unmapped fallback; empty-map fallback; case-insensitive `ACME.com` match). All `cargo test -p starter-auth-oauth --features sqlite` + `cargo build -p starter-auth-oauth --features postgres` green; `cargo build --workspace` green.
- Committed as `943a489`.

## Next

- Stage 10 picks up — final merge gate across all four shipped phases (smoke test sweep per SCOPE).

## What you need to know

- `OAUTH_SIGNUP_ENABLED=false → HTTP 403 signup_disabled` was already wired in Phase 1 (callback branch 7); this stage only adds smoke-test coverage. The existing `branch_7_signup_disabled_refuses_first_time_callback` test in `tests/callback_flow.rs` is the relevant assertion — no new test needed for that knob.
- The role-domain lookup is per-provider on purpose (one corporate domain mapping for GitHub-issued accounts can differ from the Google-issued mapping). Empty / missing map for a provider → straight to `signup_default_role`.
- Postgres feature compiles but is not exercised by a runtime smoke test in this stage (no testcontainers wiring inside starter-auth-oauth dev-deps). The SQL is structurally identical to the sqlite path and the trait contract is the same.

## Open questions

- (none)
