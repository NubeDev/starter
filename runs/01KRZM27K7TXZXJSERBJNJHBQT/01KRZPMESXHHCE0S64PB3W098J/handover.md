## Done

- Added `crates/starter-auth-oauth/src/providers/google.rs` (`GoogleProvider`) with compile-time `openid email profile` scopes, compile-time authorize/token/userinfo URLs, S256 PKCE + `prompt=select_account`, one-POST-one-GET `fetch_identity` that trusts the `email_verified` claim per R3 (returns `ProviderError::UnverifiedEmail` on `false`), `with_base_override` testing seam, and unit tests for authorize URL, id, and rewrite.
- Re-exported `GoogleProvider` from `providers/mod.rs` and `lib.rs`.
- Added smoke test `tests/two_providers_one_user.rs` (same-human-two-providers-one-user): Google signup creates Alice → GitHub callback with same verified email auto-links Branch 4 to Alice's existing user; both identity rows share `user_id`.
- `cargo test --features sqlite` green (15 unit + 10 callback + 2 identity + 1 new smoke).
- Committed as `Phase 2 — Google provider: …`.

## Next

- Stage 8 / Phase 3 — Linking, unlinking, identity listing routes (`POST /auth/oauth/{provider}/link`, `DELETE /auth/oauth/{provider}`, `GET /auth/oauth/identities`).

## What you need to know

- `KNOWN_PROVIDERS` in `config.rs` already includes `"google"` from stage 3 — no config-loader change was needed.
- Callback handler's Branch 4 (`find_by_email` hit + `email_verified=true`) does the auto-link; the smoke test exercises it across two `FakeProvider`s with different ids on a single `MemoryEverything`.
- Google's userinfo is not OIDC-verified (no `id_token` JWKS check); per SCOPE that's deferred to the Phase 5 `OidcProvider` slot.
- No real network round-trip test — production `GoogleProvider` exists but is exercised only via `FakeProvider` in tests, consistent with the GitHub stage.

## Open questions

- (none)
