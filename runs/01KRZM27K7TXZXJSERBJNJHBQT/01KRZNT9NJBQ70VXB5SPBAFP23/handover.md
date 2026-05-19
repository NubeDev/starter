## Done

- providers/github.rs: GitHubProvider with compile-time scopes `read:user user:email` and compile-time AUTHORIZE_URL / TOKEN_URL / USER_URL / EMAILS_URL. fetch_identity does POST token → GET /user → GET /user/emails, filters via pick_email (primary verified, else any verified) and returns ProviderError::UnverifiedEmail when none; access_token binding stays local (R2).
- routes/router.rs: `oauth_router::<S>(OAuthRoutesState) -> Router<S>` generic over consumer AppState; mounts /auth/oauth/{provider}/login and /auth/oauth/{provider}/callback with axum 0.8 {param} path syntax.
- routes/start.rs: random b64url-no-pad state + S256 PKCE pair, return_to validation (must start with `/`, not `//`, no scheme prefix in first segment), 302 to provider.authorize_url with flow stashed in state store.
- routes/callback.rs: seven-branch resolver — sign-in hit, link hit (same/other), link miss, verified-match link, unverified collision (409 email_already_registered), signup-enabled, signup-disabled (403). Atomic state_store.take before any provider IO. User-facing failure = `sign_in_failed` + UUID correlation_id; tracing event logs underlying reason at same correlation_id.
- session_bridge.rs: mint_session_headers wraps starter_auth_users::session::issue and emits Set-Cookie pair matching the password-login shape (R1).
- testing.rs: FakeProvider (network-free, records seen + authorize_seen) and MemoryEverything (MemoryStateStore + SqliteIdentityStore + SqliteUserStore + SqliteSessionStore on one ephemeral pool, migrations applied) — gated on feature = "sqlite" for the sqlite half.
- Cargo.toml: added axum, http, rand, base64, sha2, url = "2", and dev-deps tower, serde_urlencoded.
- tests/callback_flow.rs: nine tests covering branches 1–7 plus forged-state and provider-error paths.
- All checks pass: `cargo check --workspace --all-features --all-targets`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`. `cargo tree -p starter-auth-users | grep starter-auth-oauth` empty.
- Static "access_token never persists outside fetch_identity" grep: every hit is doc/constant/local-scope inside providers/github.rs — clean.
- Committed as `0028edd` on `codeless/starter-auth-oauth`.

## Next

- Stage 6 (REVIEW gate): one-line summary into handover that Phase 1 is end-to-end ready — Sign-in-with-GitHub works, password_not_set carries a populated providers list (Phase 1a tests already do this), sqlite-rebuild migration preserves FKs (already verified in stage 3), and the access-token-never-persists static guard passes. Do NOT proceed to Phase 2 until reviewed.

## What you need to know

- `oauth_router` mounts both routes at `/auth/oauth/{provider}/login` and `/callback`. axum 0.8 uses `{provider}` for path params (older 0.7 used `:provider`). A consumer-provided URL prefix would require wrapping with `.nest("/prefix", oauth_router(...))`; the current API does not take a prefix argument.
- The `oauth2 = "5"` dep is declared but unused — kept as a forward-compat hook for the Phase 5 generic OIDC slot. Direct reqwest calls do the v0.1 GitHub flow.
- session_bridge re-declares CSRF_COOKIE locally as "starter_csrf" because the users crate keeps that const `pub` under `routes::login` rather than re-exported; if that ever changes, swap the literal for a re-export.
- `MemoryEverything` requires `feature = "sqlite"` on starter-auth-oauth (`testing` module gates the factory portion).
- `routes::callback::resolve` is private; tests drive it indirectly via `callback_handler` because each branch is observable through HTTP response status + cookies + DB state. Future Phase 3 (`link`/`unlink`) will reuse the same OAuthRoutesState shape; do not change its public field names.
- Error response shape `{ "error": "<code>", "correlation_id": "<uuid>" }` is intentional and tests assert on the substring — keep stable.

## Open questions

- (none)
