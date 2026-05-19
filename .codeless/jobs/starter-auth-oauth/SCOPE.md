# Scope — starter-auth-oauth

> Source of truth: [`DOCS/auth/scope/SCOPE.md`](../../../DOCS/auth/scope/SCOPE.md)
> in the starter repo. This file is the per-job brief the runner
> reads before every stage; it is intentionally short. When this
> file disagrees with the source-of-truth SCOPE, that doc wins —
> open an issue and update this file.

## Goal

Add **third-party sign-up / sign-in via OAuth 2.0 / OIDC providers**
(GitHub + Google in v0.1, extensible) to a starter-based product
through a new crate that sits **next to** `starter-auth-users` and
**reuses** its `SessionStore`. OAuth is a sign-in path, not a new
`Authenticator` — once the callback resolves an identity into a
`UserRecord`, the existing auth surface (the `Authenticator` trait,
`Principal`, session cookies, bearer tokens, CSRF model) is
unchanged. A consumer who only wants password auth keeps depending
on `starter-auth-users` alone and pays nothing for OAuth.

## In scope

- New crate `starter-auth-oauth` per the source SCOPE §"Repo layout"
  (config, providers/{github,google}, state_store/{memory,sqlite,
  postgres}, identity_store/{sqlite,postgres}, routes/{router,start,
  callback,link,unlink,list}, session_bridge, error, testing seams).
- One new table `starter_auth_oauth_identities` keyed by
  composite primary key `(provider, provider_sub)` with FK to
  `starter_auth_users_users(id) ON DELETE CASCADE`.
- A SemVer-major **breaking change to `starter-auth-users`**: a new
  `LinkedProvidersLookup` trait with a `NoLinkedProviders` default,
  `UserRecord.password_hash` becomes `Option<String>`,
  `UserStore::create` takes `Option<&str>`, and `POST /auth/login`
  on `NULL` hash returns `HTTP 400 { error: "password_not_set",
  providers: [...] }` instead of `invalid_credentials`. The
  `providers` list is populated by the trait — OAuth-disabled
  builds return `[]` via `NoLinkedProviders`.
- Migration `0002_users_password_optional` shipped in
  `starter-auth-oauth`'s `migrations/` directory (NOT in
  `starter-auth-users`) so consumers who never enable OAuth never
  run it. Sqlite path is the 12-step rebuild with
  `PRAGMA foreign_key_check` before `COMMIT`; postgres path is one
  `ALTER COLUMN DROP NOT NULL`.
- Routes: `GET /auth/oauth/{provider}/login`,
  `GET /auth/oauth/{provider}/callback`,
  `POST /auth/oauth/{provider}/link`,
  `DELETE /auth/oauth/{provider}`,
  `GET /auth/oauth/identities`. `{provider}` is a path segment, so
  a typo is a 404, not a runtime parse error.
- GitHub provider with compile-time scopes `read:user user:email`
  (the `user:email` scope is load-bearing — only `/user/emails`
  returns the per-email `verified` flag R3 depends on).
- Google provider with compile-time scopes `openid email profile`
  and trust in the `email_verified` claim.
- Trait-seam `OAuthStateStore` with `MemoryStateStore` (default,
  10-minute TTL, eviction on read) plus `SqliteStateStore` and
  `PostgresStateStore` behind cargo features for multi-node.
- Trait-seam `OAuthProvider` so adding a new provider is one file
  in `providers/` plus a new config-map entry.
- Role-domain map (`OAUTH_<PROVIDER>_ROLE_DOMAIN_MAP`) on new-user
  creation, falling back to `OAUTH_SIGNUP_DEFAULT_ROLE`.
- Seven smoke tests from §"Smoke tests" in the source SCOPE — each
  is a merge gate.

## Out of scope

- **Delegated access to provider APIs.** The access token is used
  once and discarded; no refresh-token storage. A future
  `starter-auth-oauth-link` crate (Phase 6 reserved) opts into
  that.
- **Refresh-token storage / rotation.** Same reason.
- **SAML, LDAP, Kerberos.** Different threat model; separate crate
  slot (`starter-auth-saml`).
- **Multi-factor.** OAuth providers MFA on their side; in-product
  MFA is a step-up done by `starter-auth-users` (TOTP, WebAuthn)
  after the callback completes.
- **Generic OIDC discovery client.** GitHub is not OIDC; Google's
  endpoints are hard-coded. A future `OidcProvider` impl with
  `.well-known` discovery + JWKS ID-token verification is Phase 5
  reserved.
- **Admin UI** for linking / unlinking. HTTP endpoints only;
  rendering is a consumer concern.
- **User merging.** Two existing users with overlapping identities
  is an operator-run merge, not an automatic flow.
- **A new session model.** Sessions are owned by
  `starter-auth-users`. This crate calls its `SessionStore` and is
  done.
- **Provider-side account-change monitoring.** Username changes,
  account deletions, consent revocations surface on next sign-in;
  no proactive polling or webhooks.

## Hard rules (load-bearing)

Every rule below is enforceable. Trip one and the crate slides from
"a safe sign-in mechanism" into "an account-takeover vector."

- **R1** — OAuth ends in a `sas_*` session, not a new credential or
  a new authenticator. The callback handler mints the same session
  `POST /auth/login` mints; the `AuthAuthenticator` is unchanged.
- **R2** — Provider access tokens never leave the callback handler.
  Used once for `fetch_identity`, then dropped. No storage, no
  logging, no later API calls.
- **R3** — Auto-link only on a **verified** email from the provider.
  Google: `email_verified` claim must be `true`. GitHub: only
  addresses in `/user/emails` with `verified: true` count — the
  public `/user.email` field is user-edited and **never** used for
  linking. Verified-only is the only safe default; unverified
  collisions refuse with `HTTP 409 email_already_registered`.
- **R4** — Logged-in linking is a separate, explicit flow. The
  `link_mode_user_id` marker in the state-store entry tells the
  callback to add an identity row to the current user instead of
  creating one. Unlinking is refused if it would leave the user
  with no way to sign in.
- **R5** — One short-lived state store; no DB write before identity
  is known. `OAuthFlowState` records the PKCE verifier, CSRF
  `state`, `return_to`, and link-mode marker. A user who abandons
  at the provider leaves no row in `oauth_identities` or `users`.
- **R6** — Provider-specific code lives behind one
  `OAuthProvider` trait. Routes, state store, identity table, and
  linking logic stay provider-agnostic.
- **R7** — Scopes, endpoints, and required claims are compile-time
  constants per provider. Operators configure `client_id` /
  `client_secret` / `redirect_base_url`, not a custom scope string.
- **R8** — `UserRecord.password_hash` becomes `Option<String>`.
  `POST /auth/login` on `NULL` hash returns
  `{ error: "password_not_set", providers: [...] }`. The
  `providers` list flows through the new `LinkedProvidersLookup`
  trait — `NoLinkedProviders` default returns `[]` so the
  OAuth-disabled build still compiles and behaves sensibly.
- **R9** — The OAuth callback is the **only** state-changing `GET`
  in the auth surface; CSRF is provided by the OAuth `state`
  parameter (random, single-use, bound to the user's browser via
  the state-store entry). `link` and `unlink` are normal `POST` /
  `DELETE` and use the standard double-submit cookie.
- **R10** — Comments explain *why*, never *what*. No
  `// FIXED:`, no emoji, no progress logs. Doc-comments on every
  public item.

## Constraints

- One-way dependency arrow: `starter-auth-oauth → starter-auth-users
  → starter-spi`. `starter-auth-users` must not learn that
  `starter-auth-oauth` exists; the `LinkedProvidersLookup` seam is
  the only abstraction across the boundary.
- `oauth2` crate for OAuth 2.0 / PKCE / state plumbing.
  `openidconnect` is **not** pulled in for v0.1; Google's userinfo
  endpoint at sign-in cadence is one extra round trip and an
  acceptable trade.
- Migration `0002_users_password_optional` ships in the OAuth
  crate's `migrations/` (not `starter-auth-users`'s). Consumers who
  never enable OAuth never run it.
- The sqlite rebuild path is the single highest-risk operation in
  the crate. The smoke-test phase runs it against a populated
  fixture (users + active sessions + API tokens + existing OAuth
  identities) and asserts FK integrity via
  `PRAGMA foreign_key_check` before `COMMIT`.
- Provider enablement is by presence of `OAUTH_<PROVIDER>_CLIENT_ID`
  + `_CLIENT_SECRET`, not a separate `enabled` flag.
- Secrets resolve through `starter-secrets-*` if wired; env-var
  fallback otherwise.

## Phasing

Mirrors the source SCOPE §"Phasing": four ordered phases v0.1; two
reserved phases land when consumers need them.

- **Phase 1** — Crate scaffold + GitHub provider +
  `password_not_set` seam. This phase contains the
  SemVer-breaking change to `starter-auth-users` (R8); it ships in
  one phase because Phase 1 itself creates OAuth-only users, and
  without the login-error change in the same phase those users hit
  `POST /auth/login` and get a misleading `invalid_credentials`.
- **Phase 2** — Google provider. Reuses everything from Phase 1.
- **Phase 3** — Linking, unlinking, identity listing.
- **Phase 4** — Shared state store (sqlite + postgres behind cargo
  features) + signup gating (`OAUTH_SIGNUP_ENABLED=false` 403 path)
  + role-domain map per provider.
- **Phase 5 (reserved)** — Generic OIDC issuer (`OidcProvider` impl
  with `.well-known` discovery + JWKS ID-token verification).
  Lands when a consumer needs Auth0 / Okta / Keycloak.
- **Phase 6 (reserved)** — Delegated access (`starter-auth-oauth-link`
  separate crate). Persists encrypted access + refresh tokens.
  Lands when a consumer needs "call the GitHub API on the user's
  behalf."

Stage 10 (smoke tests) is the merge gate across all four shipped
phases.

## Deliverables

- `starter-auth-oauth` crate per the source SCOPE §"Repo layout".
- One SemVer-major bump on `starter-auth-users` carrying the
  `LinkedProvidersLookup` trait, `Option<String>` password_hash,
  `UserStore::create` signature change, and `password_not_set`
  login-error change.
- Two migrations: `0001_oauth_identities.sql` and
  `0002_users_password_optional.sql`, each with sqlite + postgres
  variants.
- Five routes per §"Routes": start, callback, link, unlink, list.
- GitHub provider and Google provider, each one file.
- Seven smoke tests from §"Smoke tests" passing in CI before
  merge.
- Testing seams: `FakeProvider` (deterministic, no network),
  `MemoryEverything` (one-call factory wiring memory state store +
  sqlite-in-memory identity store + `starter-auth-users`'s
  in-memory `UserStore` and `SessionStore`).

## Open questions (resolve in stage 1)

The four open questions from the source SCOPE §"Open questions",
with biases the runner records the resolved answer to under
"Decisions" before stage 3 (the first code stage) begins.

1. **Callback rate-limiting.** Bias: defer to `starter-server`
   when it grows a rate limiter; the provider already rate-limits
   the redirect.
2. **Audit log of identity changes.** Bias: emit `tracing` events
   at `info` level with structured fields (provider, user_id,
   action ∈ `{signup, link, unlink}`); durable audit table deferred
   to `starter-observability`'s audit-sink concept.
3. **Email-change-as-security-event.** Bias: emit a separate
   high-severity `tracing` event on email change vs initial set so
   an audit sink can pick it up later; structured shape deferred.
4. **First-time signup consent screen.** Bias: skip in v0.1; the
   provider's display name becomes the initial display name. A
   future `OAUTH_REQUIRE_ONBOARDING` flag inserts the step.

## Decisions

(populated in stage 1)

## Cross-cutting checks the runner must keep honest

- The **dependency-arrow test**: `starter-auth-users` does not
  depend on `starter-auth-oauth` (`cargo tree -p starter-auth-users`
  must not list this crate). The `LinkedProvidersLookup` trait is
  defined in `starter-auth-users`; the impl lives here.
- The **access-token-never-persists** static guard: a CI grep
  `grep -nrE '(access_token|access-token)' crates/starter-auth-oauth/src`
  must yield no hits outside the callback handler's `fetch_identity`
  call site. Cheap pre-check.
- The **access-token-never-persists** runtime guard: every test
  exercising the callback runs against `FakeProvider` with a
  sentinel access-token value, captures every `tracing` event and
  every SQL parameter list, and asserts the sentinel appears in
  zero of each. Substantive test.
- The **verified-email-only auto-link** test (R3): unverified
  GitHub profile email matching an existing user must produce a
  new account, not link.
- The **last-sign-in-method-refusal** test (R4 unlink): a user with
  one identity and no password gets a 409 from
  `DELETE /auth/oauth/<provider>`; the identity row remains.
- The **sqlite-rebuild-preserves-FKs** test: run
  `0002_users_password_optional` against a fixture with active
  sessions, API tokens, and existing OAuth identities;
  `PRAGMA foreign_key_check` returns no rows after the rebuild.
