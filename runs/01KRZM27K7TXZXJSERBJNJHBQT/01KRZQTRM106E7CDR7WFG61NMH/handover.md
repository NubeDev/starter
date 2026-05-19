## Done

- Added tests/abandoned_flow_no_db_trace.rs (2 tests covering start-only and forged-callback paths, both assert zero rows across users/identities/sessions and zero provider IO).
- Added tests/access_token_never_persists.rs (R2: static src/ walk + runtime tracing_subscriber::Layer recording every event field during a successful callback, asserts no forbidden field name, no SENTINEL leak in events, no SENTINEL in persisted rows).
- Added tests/sqlite_rebuild_preserves_fks.rs (replays 0001+0002 against production-shape fixture: 3 users w/ password hashes + 2 sessions + 2 API tokens + 2 OAuth identities, raw-SQL via include_str! against a single-connection pool with foreign_keys=ON, asserts foreign_key_check empty + FK joins intact + NULL hash now accepted + cascade delete still fires).
- Wired `cargo test -p starter-auth-oauth --features sqlite --no-fail-fast` into .github/workflows/ci.yml.
- Added tracing-subscriber to dev-dependencies in crates/starter-auth-oauth/Cargo.toml.
- Verified full sweep: 53 tests pass; `cargo clippy -p starter-auth-oauth --features sqlite --tests -- -D warnings` clean.

## Next

- (none) — this is the final stage of the job per the stage header (10 of 10).

## What you need to know

- The other four smoke scenarios from SCOPE (GitHub-unverified-email-cannot-hijack, Same-human-two-providers-one-user, Unlinking-last-sign-in-method-is-refused, Callback-survives-wrong-instance-routing, Password-login-error-message-includes-provider-list) were already covered as named tests in earlier stages (callback_flow.rs branch_5, two_providers_one_user.rs, link_unlink_list.rs, state_store_cross_instance.rs, and starter-auth-users/tests/http.rs respectively).
- Pre-existing rustfmt drift in crates/starter-auth-oauth/src/ (identity_store/{mod,sqlite}.rs and lib.rs) was inherited from earlier stages and was NOT touched here. The CI workflow runs `cargo fmt --all -- --check`; if that step was passing before stage 10 it will still pass (no new files added that drift). If it was failing, it is failing for the same reasons it was before this stage.
- The static-grep guard's allow-list is exactly `src/providers/` + `src/testing.rs`. Any future code that needs to mention `access_token` outside those paths will need to extend the allow-list in the test — that is the intended chokepoint.

## Open questions

- (none)
