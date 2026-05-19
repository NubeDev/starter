## Done

- routes/link.rs: POST /auth/oauth/{provider}/link — session+CSRF; stashes link_mode_user_id=Some(current_user) in OAuthFlowState; returns JSON { authorize_url } so SPA navigates same window
- routes/unlink.rs: DELETE /auth/oauth/{provider} — session+CSRF; refuses with HTTP 409 { error: "last_sign_in_method" } when no password AND no other identities; identity row preserved on refusal; deletes every (provider, sub) row for user under provider on success; idempotent
- routes/list.rs: GET /auth/oauth/identities — session-only (no CSRF, read-only per double-submit pattern); returns { identities: [(provider, email, display_name, last_login_at)] } ordered by linked_at ASC
- routes/session_guard.rs: shared session+CSRF helper; cookie parser mirrors starter_auth_users::routes::logout
- routes/start.rs: random_b64url + new_pkce_pair widened to pub(super) so link reuses them
- routes/router.rs + routes/mod.rs: wired all three new endpoints
- lib.rs: re-exports for IdentitiesResponse, IdentitySummary, LinkRequest, LinkResponse
- tests/link_unlink_list.rs: 14 tests including the smoke test `unlinking_last_sign_in_method_is_refused`; all green
- Workspace `cargo check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test -p starter-auth-oauth -p starter-auth-users --all-features` all pass
- Access-token static guard (`grep access_token crates/starter-auth-oauth/src`) unchanged — no new hits in Phase 3 code
- Committed as 3edce66 on branch codeless/starter-auth-oauth (not pushed)

## Next

- Stage 9 (Phase 4): SqliteStateStore + PostgresStateStore behind cargo features (`sqlite-state` / `postgres-state`), wire OAUTH_SIGNUP_ENABLED end-to-end (403 path), role-domain map per provider (OAUTH_<PROVIDER>_ROLE_DOMAIN_MAP), Callback-survives-wrong-instance-routing smoke test

## What you need to know

- `last_login_at` in the GET /identities response mirrors `linked_at` in v0.1; no schema change, no IdentityStore::touch_last_login. Wire shape is forward-compatible — a later stage can add a touch method and an updated_at column without breaking SPAs
- Link handler returns JSON `{ authorize_url }` (200), not 302, since the caller is a CSRF-bearing fetch from an SPA. start.rs (the unauthenticated GET /login) still returns 302
- session_guard takes an `enforce_csrf: bool` knob: link + unlink pass `true`, list passes `false`
- DELETE /auth/oauth/{provider} unlinks ALL identity rows the user has under that provider — relevant if a user ever has two GitHub accounts linked to one starter user
- Session-cookie minting reused from `session_bridge::mint_session_headers`; tests parse cookie values back out of Set-Cookie and re-emit as Cookie header

## Open questions

- (none)
