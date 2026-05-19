# Workflow — starter-auth-oauth

How to drive this job. The shape is "land a new crate that reuses
the existing session model, plus one SemVer-breaking change to
`starter-auth-users` in the same phase that creates the
OAuth-only users that change protects." Four phases; the first is
the heaviest because it carries the breaking change.

## Sequencing

- Stage 1 is **prose-only**. Resolve the four open questions in
  [SCOPE.md](./SCOPE.md), record under "Decisions", commit. No code.
- Stage 3 (Phase 1a — the breaking change to `starter-auth-users`)
  lands first because every later stage in Phase 1 depends on its
  new types. Land it as one commit so the SemVer signal is clean.
- Stages 4 + 5 (Phase 1b scaffold + Phase 1c GitHub) can be split
  across commits per file/concept but ship within Phase 1 so the
  REVIEW gate at stage 6 sees a full Sign-in-with-GitHub flow.
- Phase 2 (stage 7) is small — one file in `providers/` + one new
  smoke test. Land in a single commit.
- Phase 3 (stage 8) layers link / unlink / list on top of the
  callback handler's existing branching; the link-mode marker
  re-enters the same `callback.rs` path with a different identity
  resolution.
- Phase 4 (stage 9) adds two cargo features (`sqlite-state` and
  `postgres-state`) and the role-domain map. Each feature lands
  with its own smoke-test entry in the matrix.
- Stage 10 (smoke tests) is the merge gate. No phase ships
  individually without its own subset passing; the full sweep
  gates the final merge.

## Per-stage discipline

- Before any code change in a phase:
  - `git log -20 --oneline` for the surrounding history.
  - Read the rule numbers in [SCOPE.md](./SCOPE.md) that the stage
    touches. R1, R2, R3, R8 are the load-bearing ones; if a change
    makes any of them harder to enforce, stop and write up the
    conflict.
  - Re-read the source SCOPE's §"Flow (callback handler)" — the
    seven-step branching tree is the spec; the callback handler's
    structure mirrors it one-for-one.
  - For Phase 1a, read `crates/starter-auth-users/src/login.rs`
    and every call site of `UserStore::create` before changing the
    signature; a missed call site is a compile error but also a
    sign you should grep wider.
- Touch only what the stage names. No drive-by refactors.
- Verify before commit:
  - **Rust**: `cargo check --workspace --all-features --all-targets`,
    then `cargo test -p starter-auth-users -p starter-auth-oauth`,
    then `cargo clippy --workspace --all-targets -- -D warnings`.
  - **Migration replay**: every stage that touches a `migrations/`
    file runs the full migration sequence against a fresh sqlite
    file AND against a populated fixture (users + sessions + API
    tokens + identities) and asserts `PRAGMA foreign_key_check`
    returns zero rows.
  - **Cross-cutting**: `cargo deny check` and the dependency-arrow
    grep (`cargo tree -p starter-auth-users | grep starter-auth-oauth`
    must yield no rows).
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

Two:

- **After stage 1** — decisions sign-off before any code lands. The
  four open questions are small but two of them (audit-log scope,
  email-change event) carve out future surface in
  `starter-observability`; pin them before code paths solidify.
- **After stage 6** — Phase 1 end-to-end: Sign-in-with-GitHub
  works, the `password_not_set` login error carries a populated
  `providers` list, the sqlite-rebuild migration preserves FKs on
  the populated fixture, and the access-token-never-persists
  static + runtime guards both pass. Phases 2/3/4 land cleanly
  only if Phase 1 is right; gating here costs much less than
  rewinding from Phase 4.

Write a one-line summary into the handover at each gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for all four open questions; no code changed. |
| 3 | `starter-auth-users` SemVer bump applied: `LinkedProvidersLookup` trait + `NoLinkedProviders` default in place; `UserRecord.password_hash` is `Option<String>`; `UserStore::create` takes `Option<&str>`; every call site updated and the workspace builds; `POST /auth/login` on `NULL` hash returns `HTTP 400 { error: "password_not_set", providers: [] }` (empty list via the no-op default). |
| 4 | `starter-auth-oauth` crate compiles; `OAuthProvider` + `ProviderIdentity` + `OAuthFlowState` + `OAuthStateStore` types in place; `MemoryStateStore` round-trips an entry within TTL and evicts on read; `IdentityStore` sqlite impl + migration `0001_oauth_identities.sql` land cleanly; `OAuthLinkedProviders` impl returns the right list for a fixture user; config loads env vars + falls back through `starter-secrets-*`. |
| 5 | `providers/github.rs` exposes the trait with compile-time scopes and endpoints; `fetch_identity` calls `/user` + `/user/emails` and filters to `verified: true` addresses only; `oauth_router` mounts `start` + `callback` under a consumer-provided prefix; the callback flow's seven branches (sign-in hit, link hit, link miss, sign-in miss + verified-match, sign-in miss + unverified-collision, sign-in miss + signup enabled, sign-in miss + signup disabled) each have a test; session minted via existing `SessionStore`. |
| 7 | `providers/google.rs` lands; sign-up + sign-in via Google works end-to-end; Same-human-two-providers-one-user smoke test passes (verified-email match links GitHub to a previously Google-signed-up user). |
| 8 | `POST /link` + `DELETE /unlink` + `GET /identities` shipped behind session+CSRF; the link-mode marker in the state store routes the callback to the link branch; `DELETE` refuses with HTTP 409 if it would leave the user with no sign-in method; Unlinking-last-sign-in-method-is-refused smoke test passes. |
| 9 | `SqliteStateStore` and `PostgresStateStore` behind cargo features pass round-trip tests; `OAUTH_SIGNUP_ENABLED=false` returns HTTP 403 on first-time callbacks; role-domain map honoured on new-user creation per `OAUTH_<PROVIDER>_ROLE_DOMAIN_MAP`; Callback-survives-wrong-instance-routing smoke test passes (start on instance A, callback on instance B, with shared sqlite state store). |
| 10 | Full sweep: GitHub-unverified-email-cannot-hijack, Same-human-two-providers-one-user, Unlinking-last-sign-in-method-is-refused, Abandoned-flow-leaves-no-DB-trace, Callback-survives-wrong-instance-routing, Password-login-error-message-includes-provider-list, Provider-access-token-never-persists (static + runtime), sqlite-rebuild-preserves-FKs all pass in CI. |

## Anti-patterns

- Storing the provider access token, even "just to make the next
  test easier." R2 — used once for `fetch_identity`, then dropped.
  The static grep CI guard exists specifically to catch this; do
  not silence it.
- Linking an identity on an **unverified** provider email. R3 — the
  single decision that distinguishes a safe OAuth integration from
  an account-takeover vector. GitHub's `/user.email` is
  user-edited; only `/user/emails` `verified: true` entries count.
- Introducing a second authenticator (`OAuthAuthenticator`,
  `OidcAuthenticator`, etc.). R1 — every flow ends in the same
  `sas_*` session the password login mints; downstream code stays
  unchanged.
- A new cookie. R1 again — the existing `starter_auth_users`
  httpOnly cookie and `starter_csrf` non-httpOnly cookie are both
  reused verbatim.
- Letting `starter-auth-users` learn that `starter-auth-oauth`
  exists. The seam is the `LinkedProvidersLookup` trait defined in
  `starter-auth-users` with a `NoLinkedProviders` default; the impl
  lives in `starter-auth-oauth`. The `cargo tree` dependency-arrow
  check guards this.
- Templating scopes or endpoints from env / config. R7 — compile-
  time constants per provider. A scope diff is a code change, not
  a redeploy.
- Calling `UserStore::create` directly in the callback handler with
  `password_hash: Some(generated_random_string)`. OAuth-only users
  have `NULL` `password_hash`; the random-string approach defeats
  the whole point of the optional column and means an attacker who
  observes the random generator output could log in.
- Skipping CSRF on `/link` or `/unlink` because "they need a
  session already." R9 — only the callback `GET` uses `state` as
  CSRF; every other route uses the standard double-submit cookie.
- A sqlite migration that drops `NOT NULL` without the 12-step
  rebuild and the `PRAGMA foreign_key_check` assertion before
  `COMMIT`. The rebuild is the highest-risk operation in this
  crate; cutting corners corrupts FKs silently.
- Writing the `oauth_identities` table inside
  `starter-auth-users`'s migrations directory because "it's
  user-adjacent." That coupling is exactly what the one-way
  dependency arrow exists to prevent.
- A user-merging flow as part of v0.1. Out of scope; the crate
  refuses to combine accounts silently.
- Storing the OAuth `state` value in a cookie instead of the
  server-side `OAuthStateStore`. The PKCE verifier must not leave
  the server; `state` lives with it.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
   On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to the
   job's branch (`codeless/starter-auth-oauth`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
