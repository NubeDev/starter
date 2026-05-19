# starter-auth-oauth — Scope

## One-line summary

`starter-auth-oauth` is a small Rust crate that adds **third-party
sign-up / sign-in via OAuth 2.0 / OIDC providers** (GitHub, Google,
extensible to others) to a starter-based product. It does **not**
replace `starter-auth-users`; it sits next to it, reuses the existing
`SessionStore`, mints the same `sas_*` session cookies as the
password login does, and adds one new table (`oauth_identities`)
linking provider accounts to user records.

The crate is a **signup/login path**, not a new `Authenticator`. Once
the OAuth callback resolves an identity into a `UserRecord`, the
downstream auth model (the `Authenticator` trait, `Principal`,
session cookies, bearer tokens) is unchanged. Every consumer of the
existing auth surface continues to work without modification.

## Why this exists

`starter-auth-users` provides local email+password auth + opaque
bearer API tokens. That is sufficient for a back-office tool but not
for any product that expects a developer or end-user audience: those
users overwhelmingly prefer "Sign in with GitHub" / "Sign in with
Google" over creating a new password.

Three things need to happen for OAuth to slot into starter cleanly:

1. A new crate (this one) holds provider configuration, the start +
   callback HTTP routes, and the small bit of state needed across the
   redirect (PKCE verifier, CSRF state, return URL).
2. A new identity table links `(provider, provider_sub)` → `user_id`,
   so the same human can sign in via GitHub or Google and land on the
   same account.
3. The existing `UserRecord.password_hash` becomes optional, because
   OAuth-only users have no password.

Nothing else changes. The `Authenticator` trait stays
transport-agnostic; `Principal` stays provider-agnostic; the session
cookie shape and CSRF model are reused verbatim.

## Relationship to existing auth crates

```
starter-spi                                 (Authenticator, Principal, Role, Scope)
   ↑
   ├── starter-auth-token                   (single-owner claim token)
   ├── starter-auth-users                   (email/password + sessions + API tokens)
   └── starter-auth-oauth   ──→ depends on starter-auth-users
                                            (UserStore, SessionStore, AuthState)
```

`starter-auth-oauth` **depends on `starter-auth-users`** and reuses its
`UserStore` and `SessionStore`. It does not duplicate user storage and
does not introduce a parallel session model. The dependency arrow goes
in one direction only; `starter-auth-users` does not know
`starter-auth-oauth` exists.

A consumer enables OAuth by adding `starter-auth-oauth` to their
`Cargo.toml`, merging its router into the `ServerBuilder` alongside
the existing `auth_router`, and setting the provider env vars. A
consumer who only wants password auth depends on `starter-auth-users`
alone and pays nothing for OAuth.

## Hard rules (load-bearing)

### R1 — OAuth is a login path, not an `Authenticator`

After the callback handler exchanges the code, fetches the identity,
and resolves or creates a `UserRecord`, it **mints the same session
that `POST /auth/login` mints**: a `sas_*` opaque session id in the
`starter_auth_users` httpOnly cookie, a CSRF token in the
`starter_csrf` non-httpOnly cookie, and a row in
`starter_auth_users_sessions`. Downstream request authentication
goes through the existing `AuthAuthenticator` unchanged.

The reason: a second authenticator means a second cookie, a second
verification path, a second place to invalidate, a second source of
"why is this request authorised?" confusion. Reusing the session
means OAuth users and password users are indistinguishable to every
piece of code past the login surface — which is exactly what they
should be.

### R2 — Provider tokens never leave the callback handler

The access token returned by GitHub or Google is used **once**, to
fetch the user's identity (`/user`, `/userinfo`), and then discarded.
The crate does not store provider access tokens, does not store
refresh tokens, does not attempt to make subsequent API calls on the
user's behalf.

The reason: storing third-party tokens turns this crate into a
secrets-management problem (encryption at rest, rotation, scope
escalation surface, revocation handling) that is out of proportion to
the goal of "let users sign in." A future `starter-auth-oauth-link`
crate can opt into delegated access; the base crate stays a sign-in
mechanism.

### R3 — Account linking only on a verified email from the provider

When the callback resolves a `(provider, provider_sub)` that has no
identity row yet, the crate checks whether the provider returned a
**verified** email matching an existing user:

- **Google**: the `email_verified` claim must be `true`.
- **GitHub**: only emails returned by `GET /user/emails` with
  `verified: true` count. The public `email` field on `/user` is
  user-edited and unverified — never used for linking.

If verified-email match: auto-link the identity to the existing user.
If unverified or no match: create a new user. Account merging across
two existing users is **not** done automatically — it requires the
operator-facing link flow (see R4).

The reason: auto-linking on an unverified email is an account
takeover. Anyone who can set their GitHub profile email to your
address can claim your account. Verified-only is the only safe
default; anything more permissive is the consumer's explicit choice.

### R4 — Logged-in linking is an explicit, separate flow

A signed-in user who wants to attach a second provider hits
`POST /auth/oauth/<provider>/link`, completes the standard OAuth
dance, and the callback **adds an identity row to the current user**
instead of creating one. The link flow is recognisable to the
callback handler via a signed marker stashed in the state-store entry
at start time.

Unlinking is `DELETE /auth/oauth/<provider>` and is refused if it
would leave the user with no way to sign in (no password set AND no
other identities).

### R5 — One short-lived state store; no DB write before identity is known

OAuth requires the host to remember the PKCE verifier, CSRF `state`
value, `return_to`, and link-mode marker across the user's redirect
to the provider and back. The flow record is:

```rust
pub struct OAuthFlowState {
    pub provider:           String,            // "github" | "google" | ...
    pub state:              String,            // CSRF token, also the store key
    pub pkce_verifier:      PkceVerifier,
    pub return_to:          String,            // relative path; absolute URLs rejected
    pub link_mode_user_id:  Option<String>,    // Some(id) → /link flow; None → sign-in
    pub created_at:         DateTime<Utc>,
}
```

The crate stores these in a small **trait seam**
(`OAuthStateStore`) with two implementations:

- `MemoryStateStore` — `HashMap<state, OAuthFlowState>` behind a
  `Mutex`, 10-minute TTL, eviction on read. Default. Sufficient for
  single-node deployments.
- `SqliteStateStore` / `PostgresStateStore` — feature-gated. Required
  for multi-node deployments where the callback may hit a different
  instance than the start request.

No row is written to `oauth_identities` or `users` until the callback
has a verified identity in hand. A user who abandons the flow at the
provider leaves no database trace; the in-memory state expires.

### R6 — Provider-specific code lives behind one trait

```rust
trait OAuthProvider {
    fn id(&self) -> &'static str;                     // "github" | "google"
    fn authorize_url(&self, state: &str, pkce: &PkceChallenge) -> Url;
    async fn exchange_code(&self, code: &str, verifier: &PkceVerifier)
        -> Result<AccessToken>;
    async fn fetch_identity(&self, token: &AccessToken) -> Result<ProviderIdentity>;
}

struct ProviderIdentity {
    provider_sub: String,        // stable id from provider
    email: Option<String>,
    email_verified: bool,        // GitHub: from /user/emails; Google: from claim
    display_name: Option<String>,
}
```

Adding a new provider (GitLab, Microsoft, Apple, an enterprise OIDC
issuer) is one new `OAuthProvider` impl and a new entry in the config
map. The route handlers, the state store, the identity table, and the
linking logic are provider-agnostic.

### R7 — Static metadata; no runtime templating of redirect URLs or scopes

Each provider's scopes, userinfo endpoint, and required claim list
are **compile-time constants** in the provider impl, not runtime
config. The operator configures: `client_id`, `client_secret`,
`redirect_base_url`, and optionally an override allowlist. The crate
does not let the operator type a custom scope string and re-deploy —
that would let a misconfiguration silently request more access than
the consent screen documents.

### R8 — `password_hash` becomes optional; login route handles "use SSO" via a one-way trait seam

`starter_auth_users_users.password_hash` is migrated to nullable. A
user created via OAuth-only signup has `password_hash = NULL`. The
existing `POST /auth/login` handler, on encountering a NULL hash,
returns `HTTP 400 / { "error": "password_not_set", "providers": [...] }`
instead of the opaque `invalid_credentials`.

The dependency arrow is one-way (`starter-auth-oauth → starter-auth-users`)
so `login.rs` cannot read `oauth_identities` directly — that table
does not exist in an OAuth-disabled build. The seam is a trait
defined in `starter-auth-users`:

```rust
// in starter-auth-users
#[async_trait]
pub trait LinkedProvidersLookup: Send + Sync {
    async fn linked_providers(&self, user_id: &str) -> Result<Vec<String>>;
}

pub struct NoLinkedProviders;
#[async_trait]
impl LinkedProvidersLookup for NoLinkedProviders {
    async fn linked_providers(&self, _: &str) -> Result<Vec<String>> { Ok(vec![]) }
}

// AuthState gains:
//   pub linked_providers: Arc<dyn LinkedProvidersLookup>,
// defaulting to Arc::new(NoLinkedProviders) when OAuth is not wired.
```

`starter-auth-oauth` provides `OAuthLinkedProviders { identity_store }`
that implements the trait by querying `oauth_identities`. The consumer
wires it into `AuthState` when constructing the OAuth crate's state.
A consumer who never enables OAuth keeps the no-op default; the trait
returns an empty list; the login handler emits `password_not_set` with
`providers: []` and the UI explains the user is locked out and must
contact an admin.

**This rule is a breaking change to `starter-auth-users`'s public
API** — not just a schema change:

- `UserRecord.password_hash` → `Option<String>`.
- `UserStore::create(..., password_hash: Option<&str>, ...)`.
- Both the sqlite and postgres `UserStore` impls update accordingly.
- Every call site (login verify, admin create, tests) is updated.
- Downstream consumers that read `UserRecord.password_hash` directly
  get a compile error and adjust.

The version bump for `starter-auth-users` is the SemVer signal. The
migration that drops `NOT NULL` is one half of the change; the trait
+ `Option<String>` is the other half; they ship together.

### R9 — Adapters apply auth uniformly; OAuth does not bypass CSRF on state-changing routes

The OAuth start route is `GET` and returns a `302`. The OAuth
callback is `GET` (providers redirect here with query params) — and
is **the only state-changing GET in the auth surface**. CSRF
protection on this single route comes from the OAuth `state`
parameter itself (random, single-use, bound to the user's browser
via the state-store entry), not from the double-submit cookie used by
the rest of the auth API. Every other route in this crate
(`/link`, `/unlink`) is a normal `POST` / `DELETE` and uses the same
CSRF model as `starter-auth-users`.

### R10 — Comments explain *why*, never *what*

Same as the rest of the workspace. No `// FIXED:` banners, no emoji,
no progress logs. Doc-comments on every public item.

## Repo layout

```
crates/
  starter-auth-oauth/
    Cargo.toml
    migrations/
      0001_oauth_identities.sql        <- new table; sqlite + postgres variants
      0002_users_password_optional.sql <- ALTERs starter_auth_users_users
    src/
      lib.rs
      config.rs                        <- OAuthConfig, ProviderConfig (env-loaded)
      provider.rs                      <- OAuthProvider trait + ProviderIdentity
      providers/
        github.rs                      <- impl OAuthProvider for GitHub
        google.rs                      <- impl OAuthProvider for Google
      state_store/
        mod.rs                         <- trait OAuthStateStore + OAuthFlowState
        memory.rs                      <- in-process HashMap impl (default)
        sqlite.rs                      <- feature = "sqlite"
        postgres.rs                    <- feature = "postgres"
      identity_store/
        mod.rs                         <- trait IdentityStore
        sqlite.rs
        postgres.rs
      routes/
        router.rs                      <- oauth_router::<S>(state) -> Router<S>
        start.rs                       <- GET  /auth/oauth/{provider}/login
        callback.rs                    <- GET  /auth/oauth/{provider}/callback
        link.rs                        <- POST /auth/oauth/{provider}/link
        unlink.rs                      <- DELETE /auth/oauth/{provider}
        list.rs                        <- GET  /auth/oauth/identities (me)
      session_bridge.rs                <- mints sas_* session via SessionStore
      error.rs
    tests/
      memory_state_store.rs
      callback_link_flow.rs
      verified_email_only.rs
```

## Data model additions

Column types align with the existing `starter-auth-users` convention:
`TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP` for timestamps (sqlite
stores ISO-8601 strings; the postgres variant uses
`TIMESTAMPTZ NOT NULL DEFAULT now()`).

```sql
-- crates/starter-auth-oauth/migrations/0001_oauth_identities.sql  (sqlite)
CREATE TABLE starter_auth_oauth_identities (
  provider      TEXT NOT NULL,           -- 'github' | 'google' | ...
  provider_sub  TEXT NOT NULL,           -- stable id from provider
  user_id       TEXT NOT NULL
                  REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
  email         TEXT,                    -- snapshot, may drift
  display_name  TEXT,
  created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_login_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (provider, provider_sub)
);
CREATE INDEX idx_oauth_identities_user
  ON starter_auth_oauth_identities (user_id);
```

```sql
-- crates/starter-auth-oauth/migrations/0002_users_password_optional.sql (sqlite)
-- Sqlite cannot drop NOT NULL via ALTER. This is a 12-step rebuild
-- and is the riskiest single change in this crate — exercise it
-- against a production-shape dataset before merge.
PRAGMA foreign_keys = OFF;
BEGIN;
CREATE TABLE starter_auth_users_users__new (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL UNIQUE,
  password_hash TEXT,                                  -- was NOT NULL
  role          TEXT NOT NULL,
  created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
  -- ...other columns mirrored verbatim from 0001_users.sql
);
INSERT INTO starter_auth_users_users__new
  SELECT id, email, password_hash, role, created_at /*, ... */
  FROM starter_auth_users_users;
DROP TABLE starter_auth_users_users;
ALTER TABLE starter_auth_users_users__new RENAME TO starter_auth_users_users;
-- starter_auth_users_sessions and starter_auth_users_tokens already
-- reference users(id); sqlite preserves their FK definitions across the
-- table rename, but the migration verifies with PRAGMA
-- foreign_key_check before committing.
COMMIT;
PRAGMA foreign_keys = ON;
```

```sql
-- crates/starter-auth-oauth/migrations/0002_users_password_optional.sql (postgres)
ALTER TABLE starter_auth_users_users
  ALTER COLUMN password_hash DROP NOT NULL;
```

The `oauth_identities` table is **owned by `starter-auth-oauth`**, not
by `starter-auth-users`. Keeping it in the OAuth crate's migration
folder means a consumer who never enables OAuth never runs that
migration.

The `users_password_optional` migration is the one place the OAuth
crate reaches into `starter-auth-users`'s schema. It is documented in
that crate's CHANGELOG as a load-bearing forward-migration that
remains correct whether or not OAuth is enabled (a NULL
`password_hash` simply means "password login disabled for this user",
which is already the natural reading). The sqlite rebuild is the
single highest-risk operation in the crate; the smoke-test phase
runs it against a populated fixture with active sessions and tokens
to confirm FK integrity survives.

## Routes

| Method | Path                                       | Purpose                                              | Auth         |
| ------ | ------------------------------------------ | ---------------------------------------------------- | ------------ |
| GET    | `/auth/oauth/{provider}/login`             | Start sign-in: 302 to provider authorize URL         | none         |
| GET    | `/auth/oauth/{provider}/callback`          | Exchange code, resolve identity, mint session, 302   | none         |
| POST   | `/auth/oauth/{provider}/link`              | Start link-mode for current user                     | session+CSRF |
| DELETE | `/auth/oauth/{provider}`                   | Unlink the current user's identity for `{provider}`  | session+CSRF |
| GET    | `/auth/oauth/identities`                   | List the current user's linked identities            | session      |

`{provider}` is a path segment, not a query parameter, so the
provider is part of the route shape and a typo is a 404 rather than
a runtime "unknown provider" string parse.

The router is generic over the consumer's `AppState`:

```rust
pub fn oauth_router<S>(state: OAuthState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
```

mirroring `auth_router` in `starter-auth-users`. The consumer's
`ServerBuilder` merges both routers under the same `/auth` prefix.

## Configuration

```
OAUTH_BASE_URL=https://app.example.com         # absolute base for redirect URLs
OAUTH_GITHUB_CLIENT_ID=...                     # presence enables the provider
OAUTH_GITHUB_CLIENT_SECRET=...
OAUTH_GOOGLE_CLIENT_ID=...
OAUTH_GOOGLE_CLIENT_SECRET=...
OAUTH_STATE_STORE=memory                       # memory | sqlite | postgres
OAUTH_SIGNUP_DEFAULT_ROLE=Reader               # role assigned to new OAuth signups
OAUTH_SIGNUP_ENABLED=true                      # false = sign-in only, no new users
```

Secrets resolution goes through `starter-secrets-*` if the consumer
has wired one (e.g. `starter-secrets-keyring`). Otherwise env-var
fallback. Presence of `{PROVIDER}_CLIENT_ID` + `{PROVIDER}_CLIENT_SECRET`
is what enables a provider — there is no separate `enabled` flag.

## Flow (callback handler)

1. Parse `state` and `code` from query string. Reject if either
   missing.
2. Look up `state` in `OAuthStateStore`; reject if absent or expired.
   Atomic remove on hit (single-use).
3. `provider.exchange_code(code, flow_state.pkce_verifier)` →
   `AccessToken`.
4. `provider.fetch_identity(token)` → `ProviderIdentity`.
5. `IdentityStore::find(provider, identity.provider_sub)`:
   - **Hit, sign-in flow**: load user, update `last_login_at`,
     proceed to step 6.
   - **Hit, link flow**: refuse if the identity belongs to a
     different user; otherwise proceed (idempotent).
   - **Miss, link flow**: insert identity row pointing at the
     current logged-in user.
   - **Miss, sign-in flow + verified email matches existing user**:
     insert identity row pointing at that user (R3).
   - **Miss, sign-in flow + unverified email collides with an
     existing user's email**: refuse with
     `HTTP 409 / { "error": "email_already_registered",
     "message": "An account with this email exists. Sign in with
     your original method, then link <provider> from settings." }`.
     No user is created; no identity row is written. This is the
     branch that defeats the unverified-email account-takeover
     vector — without it, `users.email`'s `UNIQUE NOT NULL`
     constraint would surface as a generic 500 from `UserStore::create`.
   - **Miss, sign-in flow + no match + `OAUTH_SIGNUP_ENABLED`**:
     create user (role = `OAUTH_SIGNUP_DEFAULT_ROLE`), insert
     identity row. The provider's email is snapshotted onto the new
     user record verbatim (verified or not — it is the user's only
     contactable address); whether the consumer subsequently runs an
     email-verification flow against it is out of scope for this
     crate.
   - **Miss, sign-in flow + signup disabled**: 403 with
     "sign-up is disabled; ask an administrator to invite you."
6. Mint a session via `SessionStore`, set `starter_auth_users` and
   `starter_csrf` cookies, 302 to `flow_state.return_to`.

Every step that fails returns a deliberate, non-leaky error: the user
sees "sign-in failed" with a correlation id; the server logs the
underlying reason with the provider id and the `state` value. The
correlation id lets an operator grep logs without exposing internals
to the user.

## Testing seams

- `starter-auth-oauth::testing::FakeProvider` — an `OAuthProvider`
  impl that does not hit the network. The test sets up the
  `ProviderIdentity` the fake will return; `exchange_code` and
  `fetch_identity` are deterministic. Used by every non-network test.
- `starter-auth-oauth::testing::MemoryEverything` — a one-call
  factory returning `OAuthState` wired with `MemoryStateStore` + a
  sqlite-in-memory `IdentityStore` + `starter-auth-users`'s
  in-memory `UserStore` and `SessionStore`. Drop into a `TestApp` and
  exercise full callback flow.
- HTTP-level tests use `starter-server::testing::TestApp`; the
  router is mounted under `/auth` alongside `auth_router`.

## Smoke tests (before merging)

### "GitHub unverified email cannot hijack" test

Existing user `alice@example.com` has password auth. A second user
sets their GitHub profile email to `alice@example.com` (no
verification). They click "Sign in with GitHub." A new account is
created with `password_hash = NULL` and no link to alice's user. If
the callback links the new identity to alice's user, R3 has slipped.

### "Same human, two providers, one user" test

Alice signs up via Google with `alice@example.com` (verified). Later,
signed out, she clicks "Sign in with GitHub" and her GitHub account
returns `alice@example.com` from `/user/emails` with `verified: true`.
The callback finds no `(github, gh_sub)` identity, finds the
verified-email match, and links the GitHub identity to her existing
user. She is signed into the same account. If a second user is
created instead, the linking rule has slipped.

### "Unlinking last sign-in method is refused" test

A user has one Google identity and no password. They call
`DELETE /auth/oauth/google`. The endpoint refuses with a clear
message; the identity row remains; the user is still signed in. If
the unlink succeeds, the user has been locked out of their own
account.

### "Abandoned flow leaves no DB trace" test

Open the start route, observe the `OAuthStateStore` has one entry,
abandon the redirect (no callback). Wait past the TTL. The store
entry is gone; no row exists in `oauth_identities`; no row exists in
`users`.

### "Callback survives wrong-instance routing in multi-node" test

With `OAUTH_STATE_STORE=sqlite` (shared DB), start the flow on
instance A and callback on instance B. Flow completes normally.
With `memory`, the same scenario fails fast with a clear "state not
found" — not a 500.

### "Password login error message includes provider list" test

Alice has `password_hash = NULL` and a linked Google identity. She
hits `POST /auth/login` with her email and a guessed password. The
response is a 400 with body
`{"error": "password_not_set", "providers": ["google"]}`, not a
generic "invalid credentials." If the UI cannot tell the user
"use Google to sign in," the change to `password_hash` nullability
was wasted.

### "Provider access token never persists" check (static + runtime)

Two parts:

- **Static CI guard**: `grep -nrE '(access_token|access-token)' crates/starter-auth-oauth/src` rejects any storage or logging
  reference outside the callback handler's `fetch_identity` call
  site. Runs in CI; cheap; catches casual regressions.
- **Runtime test**: the callback flow runs end-to-end against
  `FakeProvider` while a recording subscriber captures every
  `tracing` event (including span fields) and every SQL statement
  the test `UserStore` / `IdentityStore` see. After the flow
  completes, the test asserts the literal access-token bytes
  (a known sentinel from `FakeProvider`) appear in zero logged
  fields and zero SQL parameter lists. This is the substantive
  test; the grep is the cheap pre-check.

## Non-goals

- **Not delegated access to provider APIs.** The crate fetches the
  user's identity once at sign-in and discards the access token. Apps
  that need to call the GitHub API on the user's behalf later build
  on top of this crate or use a separate one.
- **Not refresh-token storage / rotation.** Same reason as above; the
  base crate is sign-in only.
- **Not SAML, not LDAP, not Kerberos.** This crate is OAuth 2.0 / OIDC
  only. Enterprise SSO over SAML is a separate crate slot
  (`starter-auth-saml`) and a different threat model.
- **Not multi-factor.** OAuth providers MFA on their side; this crate
  trusts their assertion. A consumer who needs in-product MFA on top
  of OAuth wires it in `starter-auth-users` (TOTP, WebAuthn) as a
  step-up after the OAuth callback completes.
- **Not a generic OIDC discovery client.** GitHub is not OIDC at all;
  Google is OIDC but the crate hard-codes its endpoints rather than
  fetching `/.well-known/openid-configuration` at boot. A consumer
  who needs arbitrary OIDC issuers (Auth0, Okta, Keycloak) gets a
  future `OidcProvider` impl that *does* do discovery — opt-in.
- **Not an admin UI.** Linking, unlinking, and identity listing are
  HTTP endpoints. Rendering them in a settings page is a consumer
  concern.
- **Not user merging.** If two existing users discover they have
  overlapping identities, an operator-run merge is out of scope.
  The crate refuses to silently combine accounts.
- **Not a session model.** Sessions are owned by `starter-auth-users`.
  This crate produces a session via that crate's `SessionStore` and
  is done.
- **Not provider-side account changes.** If a user changes their
  GitHub username, deletes their Google account, or revokes the
  app's consent, the next sign-in attempt fails gracefully but the
  crate does not proactively monitor for these events.

## Decisions made

- **Library choice: `oauth2` crate.** De-facto Rust OAuth 2.0 client;
  supports PKCE and state out of the box; async-first; works with
  `reqwest 0.12` already in the workspace. `openidconnect` is *not*
  pulled in for v0.1 — Google's userinfo endpoint gives us the same
  information as ID-token verification at one extra round trip, which
  is acceptable at sign-in cadence. We revisit if a real consumer
  shows up needing offline ID-token verification.
- **Provider list for v0.1: GitHub + Google.** These are 90% of
  consumer demand. The trait is provider-agnostic; adding GitLab,
  Microsoft, Apple, or arbitrary OIDC issuers is one file in
  `providers/`. GitHub's compile-time scope set is
  `read:user user:email` — `user:email` is required so
  `GET /user/emails` returns the per-email `verified` flag that R3
  depends on for safe auto-linking. Google's scope set is
  `openid email profile`.
- **Identity table owns `(provider, provider_sub)` as a composite
  primary key**, not a surrogate id. The pair is already globally
  unique (a provider guarantees `sub` uniqueness within itself) and
  using it as PK makes the "is this identity already linked?" lookup
  a single primary-key probe.
- **OAuth ends in a `sas_*` session, not a new credential.** Reasons
  in R1. Side benefit: `/auth/me`, role/scope enforcement, CSRF
  double-submit, session invalidation, and the existing axum
  extractor stack all work without modification.
- **`OAuthStateStore` is a trait, default `memory`.** The vast
  majority of consumers run a single instance and do not need a
  shared store. The seam exists so multi-node deployments are a
  config change, not a refactor.
- **Verified email is mandatory for auto-link.** Reasons in R3. This
  is the single decision that distinguishes a safe OAuth integration
  from an account-takeover vector.
- **Provider access tokens are never persisted.** Reasons in R2.
  Re-opening this decision requires writing a threat model first.
- **`password_hash` becomes optional.** The migration ships in this
  crate, not in `starter-auth-users`, so consumers who never enable
  OAuth never run it.
- **The callback is `GET`.** This is not the standard "no state-changing
  GETs" rule from REST, because the `state` parameter is the CSRF
  token here — single-use, server-generated, bound to a server-side
  flow record. Reasons in R9.
- **Scopes are compile-time constants per provider.** Operators
  configure who (`client_id` / `client_secret`), not what (the scope
  list). Reasons in R7.
- **Signup is enabled by default, disableable by env.** Many
  consumers want OAuth as both signup and signin. The few who want
  signin-only (closed-beta products, internal tools) flip
  `OAUTH_SIGNUP_ENABLED=false` and get a 403 with a clear message on
  first-time callbacks.
- **Display name and email are lazily snapshotted on each sign-in.**
  The identity row stores email and display name as the provider
  returned them on the most recent successful sign-in. The crate
  does not poll, does not webhook, does not refresh between
  sign-ins. A name change on the provider takes effect the next
  time the user signs in via that provider — no sooner.

## Open questions

- **Rate-limiting the callback endpoint.** The provider has already
  rate-limited the redirect, but a flood of garbage `state` values
  is a cheap DoS. Probably belongs in `starter-server`'s rate
  limiter, not here. Decide when `starter-server` grows one.
- **Audit log of identity changes.** Linking, unlinking, and
  first-time signups are security-relevant events. They should
  emit `tracing` events at `info` level with structured fields;
  whether to also write them to a durable audit table is deferred
  until `starter-observability` has an audit-sink concept.
- **Email change detection as a security event.** The snapshot
  policy above silently updates `oauth_identities.email` on the
  next sign-in. Should a *change* (vs initial set) also emit a
  high-severity `tracing` event for an audit sink to pick up?
  Lean yes; defer the structured shape until `starter-observability`
  has an audit-sink concept.
- **Where to render the consent / first-time signup screen.** Some
  products want a "Welcome, choose your username" step between the
  callback and the redirect to `return_to`. v0.1 skips this; the
  user's provider display name becomes their initial display name
  and they can edit it later. A future `OAUTH_REQUIRE_ONBOARDING`
  flag could insert the step.

## Phasing

Each phase is independently mergeable. Stopping after any phase
leaves a working product.

### Phase 1 — Crate scaffold + GitHub provider + `password_not_set` seam

The `password_not_set` change and the `LinkedProvidersLookup` trait
land in **this** phase, not later. Reason: Phase 1 itself creates
OAuth-only users (verified-email auto-link + new-user creation are
both in Phase 1's outcome). Without the login-error change in the
same phase, those users hit `POST /auth/login` and get a misleading
`invalid_credentials` until Phase 2 ships. The seam is small and
ships alongside its only justification.

- `starter-auth-oauth` crate, Cargo.toml, lib.rs.
- `OAuthProvider` trait, `OAuthFlowState`, `ProviderIdentity`,
  error type.
- `providers/github.rs` with compile-time scopes + endpoints.
- `MemoryStateStore`.
- `IdentityStore` trait + sqlite impl.
- `0001_oauth_identities.sql` (sqlite + postgres variants).
- `0002_users_password_optional.sql` (sqlite rebuild + postgres
  ALTER, with the FK-integrity assertion on the sqlite path).
- **In `starter-auth-users`**: `LinkedProvidersLookup` trait +
  `NoLinkedProviders` default + `AuthState` field +
  `UserRecord.password_hash: Option<String>` +
  `UserStore::create` signature update +
  `POST /auth/login` returns `password_not_set` on NULL hash.
  This is the SemVer-major bump for `starter-auth-users`.
- **In `starter-auth-oauth`**: `OAuthLinkedProviders` impl wired
  into the consumer's `AuthState` at construction.
- Routes: `start`, `callback`. No link / unlink / list yet.
- Verified-email-only auto-link (R3) and email-collision refusal
  (R3 / Flow step 5).
- Session mint via existing `SessionStore`.
- `FakeProvider` testing seam + smoke tests: verified-email,
  unverified-email-cannot-hijack, abandoned-flow,
  password-login-error-message, access-token-never-persists
  (static + runtime), sqlite-rebuild-preserves-FKs.

Outcome: a consumer can add `starter-auth-oauth`, set two env vars,
and "Sign in with GitHub" works end-to-end — including the
"sign in with GitHub" hint surfaced from password login. New users
get a `Reader` role; verified-email matches link to existing users;
unverified-email collisions refuse cleanly.

### Phase 2 — Google provider

- `providers/google.rs` with compile-time scopes + endpoints.
- No new infrastructure; reuses everything from Phase 1.
- Smoke test: "same human, two providers, one user" (becomes
  meaningful once a second provider exists).

Outcome: full sign-up and sign-in via either provider.

### Phase 3 — Linking, unlinking, identity listing

- `POST /auth/oauth/{provider}/link` (link-mode marker in state
  store, session+CSRF).
- `DELETE /auth/oauth/{provider}` with the last-method check
  (session+CSRF).
- `GET /auth/oauth/identities` for the current user.
- Smoke test: "unlinking last sign-in method is refused."

Outcome: a signed-in user can manage their linked providers.

### Phase 4 — Shared state store + signup gating + role mapping

Role mapping is **decided**: a `ProviderConfig.role_domain_map:
HashMap<String, Role>` (default empty). On new-user creation, the
crate checks the verified email's domain against the map and
assigns the matched role; on no match (or empty map, or unverified
email), `OAUTH_SIGNUP_DEFAULT_ROLE` applies. Phase 4 ships the
implementation and the `OAUTH_GITHUB_ROLE_DOMAIN_MAP` /
`OAUTH_GOOGLE_ROLE_DOMAIN_MAP` env var format (comma-separated
`domain=Role` pairs).

- `SqliteStateStore` + `PostgresStateStore` behind cargo features.
- `OAUTH_SIGNUP_ENABLED=false` path with the clear-403 message.
- Domain-allowlist role mapping per the decision above.
- Smoke tests: "callback survives wrong-instance routing,"
  "domain map assigns Writer to `@acme.com` signups."

Outcome: multi-node deployments, closed-beta products, and
domain-tiered role assignment are first-class.

### Phase 5 (reserved) — Generic OIDC issuer

- `OidcProvider` impl that performs `.well-known/openid-configuration`
  discovery and verifies ID tokens via JWKS.
- Enables Auth0 / Okta / Keycloak / arbitrary issuers from one
  config entry per issuer.
- Pulls in `openidconnect` crate.

Lands when a consumer needs it.

### Phase 6 (reserved) — Delegated access (`starter-auth-oauth-link`)

- Separate crate. Persists provider access + refresh tokens
  (encrypted via `starter-secrets-*`).
- Exposes `get_provider_client(user_id, provider) -> ApiClient`.
- Out of scope for the base crate by R2.

Lands when a consumer needs "call the GitHub API on the user's
behalf."

## Bottom line

**One new crate, one new table, one nullable column. OAuth is a
sign-in path that ends in the same session cookie the password
login already mints — so every downstream auth check is unchanged.
GitHub and Google in v0.1; the provider seam is one trait, so the
next provider is one file. Verified-email-only auto-linking, no
provider tokens stored, and an explicit linking flow for users who
want more than one identity on one account.**
