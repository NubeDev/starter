# starter-auth-users — Signup scope

## One-line summary

Add a **self-service email+password signup path** to
`starter-auth-users`: one new route (`POST /auth/signup`), one new
optional table (`starter_auth_users_invites`), and a small env-driven
gate so an operator can choose between *open signup*, *invite-only*,
or *signup-disabled*. On success the route mints the same `sas_*`
session the existing `POST /auth/login` mints, so the user is signed
in immediately and every downstream auth check is unchanged.

The route lives in `starter-auth-users` (no new crate). It reuses the
existing `password` module for hashing, the existing `UserStore` for
persistence, and the existing `SessionStore` for session minting. It
does **not** introduce a new `Authenticator`, a new cookie, a new
principal type, or a parallel session model.

## Why this exists

Today a new user enters the system through exactly two paths:

1. **`starter-auth-users` admin CLI** (`create-admin`) — an operator
   runs a command on the box.
2. **`starter-auth-oauth` callback** — the user clicks "Sign in with
   GitHub / Google" and a row is auto-created.

There is no way for a human with a browser and an email address to
create their own local account. That is fine for an internal tool
where every user is provisioned by an admin, and fine for a product
where every user signs in via GitHub. It is a dead end for:

- products that want email+password as a first-class option alongside
  OAuth (the most common consumer SaaS shape);
- closed-beta products that gate access via single-use invite codes
  the operator hands out;
- air-gapped or on-prem deployments where OAuth providers are not
  reachable but the operator still wants users to self-onboard
  instead of every account being CLI-created.

The work is small — one handler, one env-gated branch, one optional
invite table — but it has to be designed deliberately because signup
is the one un-authenticated route that *writes* to `users`. Every
hard rule below exists to keep that write from becoming an abuse
vector.

## Relationship to existing auth crates

```
starter-spi                                   (Authenticator, Principal, Role)
   ↑
   ├── starter-auth-token
   ├── starter-auth-users                     (login, logout, me, admin,
   │     └── signup (this scope)               + signup, + invites)
   └── starter-auth-oauth ──→ depends on starter-auth-users
```

No new crate. No new SPI trait. No new dependency arrow. The
signup route is one more handler in `crates/starter-auth-users/src/routes/`
and the optional invite table is one more migration in that crate.

A consumer who only wants admin-provisioned accounts disables signup
(`SIGNUP_MODE=disabled`, the default) and the route returns `404`
from the router — it is not mounted. A consumer who wants invite-only
flips `SIGNUP_MODE=invite` and hands out codes from
`POST /auth/invites` (admin-only). A consumer who wants open signup
flips `SIGNUP_MODE=open`.

## Hard rules (load-bearing)

### R1 — Signup ends in the same `sas_*` session login mints

After the handler hashes the password, inserts the user row, and
(invite mode) marks the invite consumed, it **mints a session via
`SessionStore::create` exactly the way `POST /auth/login` does**:
`starter_auth_users` httpOnly cookie, `starter_csrf` non-httpOnly
cookie, response body carrying the CSRF token. The user is signed
in on the next request without a separate login round-trip.

The reason: a signup that does not auto-login forces the SPA to chain
`POST /auth/signup` → `POST /auth/login` and handle the two-step
failure modes (signup ok, login race-failed, what now?). Reusing the
exact session mint of the login route eliminates that branch and
keeps the surface symmetric with OAuth, which already mints a session
on first-time signup via its callback (see auth/scope/SCOPE.md R1).

### R2 — Signup is off by default; the mode is a single env switch

```
SIGNUP_MODE=disabled    # default; the route 404s
SIGNUP_MODE=invite      # signup requires a valid, unused invite code
SIGNUP_MODE=open        # anyone with an email + password can sign up
```

The default is `disabled` because the current product shape (admin
provisions everyone, or OAuth handles signup) is the safe one to
keep working unchanged. A consumer who wants signup must opt in
explicitly. There is no "automatic" mode that infers intent from
other config.

`disabled` does not mount the route at all (`auth_router` skips it)
rather than mounting it and returning 403. Mounting a disabled
endpoint is a footgun: it shows up in OpenAPI, it shows up in
scanners, and it implies "this exists but is rejecting you."

### R3 — Invite codes are single-use, expiring, and bound to an email at issue time

The invite row schema:

```sql
CREATE TABLE starter_auth_users_invites (
  code         TEXT PRIMARY KEY,            -- opaque ulid + '_' + 22-char base32 random
  email        TEXT NOT NULL,               -- the invite is for this address only
  role         TEXT NOT NULL,               -- role assigned on signup
  issued_by    TEXT NOT NULL                -- admin user id
                 REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
  issued_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at   TEXT NOT NULL,               -- ISO-8601; default 7 days
  consumed_at  TEXT,                        -- NULL until signup; set atomically
  consumed_by  TEXT                         -- user id of the resulting account
                 REFERENCES starter_auth_users_users(id) ON DELETE SET NULL
);
CREATE INDEX idx_invites_email ON starter_auth_users_invites (email);
```

Three rules ride on this shape:

- **Bound to email at issue time.** The signup request's `email`
  must equal the invite's `email` (case-insensitive, NFC-normalised).
  An invite for `alice@example.com` cannot be used to sign up
  `bob@example.com`. This stops a leaked invite from being a free
  account; the attacker would also need control of the bound mailbox
  to receive the invite in the first place.
- **Single-use, atomically.** Consumption is a single SQL statement:
  `UPDATE … SET consumed_at = now, consumed_by = :user_id WHERE code
  = :code AND consumed_at IS NULL`. If `rows_affected == 0` the
  signup is refused — that branch catches both "already used" and
  "expired between fetch and update." A second request with the same
  code loses the race deterministically.
- **Time-bounded.** `expires_at` defaults to 7 days from issue. The
  handler checks `expires_at > now` before attempting consumption;
  expired codes return `410 Gone` with `{"error":
  "invite_expired"}`, distinguishing "this code was real but is
  stale" from "this code never existed" (`404` /
  `{"error": "invite_not_found"}`).

The reason for atomic single-use: the password-hash + user-insert +
invite-consume sequence is not in one transaction across all
backends (sqlite vs postgres semantics differ for cross-table
transactions in the existing store impls), and a non-atomic check-
then-consume is a textbook race. The single conditional UPDATE is
the smallest correct primitive.

### R4 — Email-already-registered is a deliberate, non-leaky 409

If `UserStore::create` returns `Conflict`, the handler responds:

```
HTTP 409
{ "error": "email_already_registered" }
```

It does **not** disclose whether the existing account is
password-based or OAuth-only. The SPA's signup form on a 409 routes
the user to the login screen with the email pre-filled; the login
screen's existing `password_not_set` response (with `providers`
list) handles the "this is an OAuth account" case.

The reason: leaking "this email exists and is OAuth-only" from an
unauthenticated endpoint is an account-enumeration oracle. The
existing login route already discloses the same fact, but only to
someone who knows (or guessed) the password — that gate is the load-
bearing difference. Signup is gateless by definition, so it must not
leak.

For the same reason, in `SIGNUP_MODE=invite`, the handler returns
`401 / {"error": "invite_required"}` for *missing* or *invalid*
codes uniformly — it does not distinguish "no such code" from
"code exists but for a different email" to an unauthenticated
caller. The granular `invite_not_found` / `invite_expired`
distinction above is for the **admin** invite-management endpoints
(`GET /auth/invites`), not for the signup handler's response.

### R5 — Password policy is enforced server-side and identical to admin-create

The signup handler validates:

- email parses (`email_address` crate or equivalent) and is ≤ 254
  chars;
- password length is `>= SIGNUP_PASSWORD_MIN_LEN` (default 12,
  configurable) and `<= 4096` (the argon2 input upper bound the
  workspace already uses);
- password is not literally equal to the email local-part or to a
  small embedded list of the top ~100 breached passwords (single
  `OnceCell<HashSet<&'static str>>`, baked at compile time from a
  vendored file; no network).

Validation failures return `400` with
`{"error": "weak_password" | "invalid_email", "message": "…"}`.
The same validation function is exported and called by
`admin::create_admin` and by any future admin "create user" endpoint,
so the policy is enforced in exactly one place.

The reason for keeping the policy minimal and shipping no zxcvbn-
class library: the workspace already takes a hard "no heavy
dependencies for one feature" line, and a 12-char minimum plus a
top-100 blocklist defeats >99% of credential-stuffing pre-attempts
without dragging in 3 MB of dictionaries. Consumers who want
zxcvbn-grade strength estimation wire it on the client (their SPA),
not in this crate.

### R6 — Signup is rate-limited per IP and per email; both checks must pass

The handler consults a `SignupRateLimiter` trait seam **before**
hashing the password (Argon2id is deliberately expensive — a flood
of signup requests is a CPU-DoS vector otherwise):

```rust
#[async_trait]
pub trait SignupRateLimiter: Send + Sync {
    /// Returns Ok(()) if the request may proceed, Err with a
    /// Retry-After (seconds) otherwise.
    async fn check(&self, ip: IpAddr, email_normalised: &str)
        -> Result<(), RateLimited>;
}

pub struct RateLimited { pub retry_after_secs: u32 }
```

Two implementations ship:

- `MemoryRateLimiter` — token bucket per `(ip)` + per
  `(email_normalised)`, 5 requests / 10 minutes each. Default.
- `NoRateLimit` — for tests and for consumers who put a real WAF /
  reverse-proxy limiter in front. Opt-in via
  `SIGNUP_RATE_LIMIT=disabled`.

On exhaustion the handler returns `429` with
`Retry-After: <secs>` and `{"error": "rate_limited"}`. Both
buckets are checked; whichever has less budget left determines the
`Retry-After`. The check is performed before any DB work and before
password hashing.

The reason for two buckets: per-IP defeats a single attacker
brute-forcing many emails; per-email defeats a botnet trying many
IPs against one target email (to lock its owner out of the
"register this email" capability ahead of them, or to flood a
specific mailbox with verification mail if Phase 2 ships).

### R7 — Email verification is a separate phase, not a blocker for v0.1

A user created via signup has `email_verified = false` (a new column;
default `0`). In Phase 1 the column exists and is set on signup but
**no route gates on it** — verified and unverified users have the
same capabilities. The flag is recorded so Phase 2 can add the
`/auth/verify-email` flow without a schema migration in a hot path.

The reason for splitting it: email verification requires an outbound
mail dependency (SMTP / SES / Postmark), which is a separate
operator-facing setup (`MAIL_TRANSPORT=…`, sender domain, SPF/DKIM)
that this scope is not going to design. Shipping signup without
verification is correct for many products (closed-beta with invite
codes already verifies the email implicitly; open-signup products
often add verification as a separate user-facing step). Shipping the
column on day one means Phase 2 is purely additive.

The OAuth crate's `oauth_identities` table already records a
verified-email signal from the provider; the new column is for
**locally-signed-up** users and is independent.

### R8 — Signup never elevates role; admin role assignment is admin-only

Open-mode signup hard-codes the role to `SIGNUP_DEFAULT_ROLE`
(default: `Reader`). The signup request body **does not** accept a
`role` field; if one is sent, it is ignored without error (treated as
unknown JSON field per serde's default).

Invite-mode signup uses the role baked into the invite at issue
time. The invite-issue endpoint (`POST /auth/invites`) is
admin-gated by existing `require_role(Admin)` middleware. The
operator picks the role; the user cannot override it.

The reason: any path that lets an unauthenticated request influence
its own `Role` is a privilege-escalation oracle. Hard-coding the
default and putting role choice behind admin auth keeps the
invariant simple: **no role change without an `Admin` in the
loop**.

### R9 — The route is mounted conditionally; admin endpoints are gated by `Admin`

`auth_router` reads `AuthState.signup` (new field):

```rust
pub enum SignupMode {
    Disabled,                            // route not mounted
    Open    { default_role: Role },
    Invite  { /* uses invite.role */ },
}

pub struct AuthState {
    // ... existing fields
    pub signup: SignupMode,                                // default: Disabled
    pub invites: Option<Arc<dyn InviteStore>>,             // Some(_) iff mode=Invite
    pub rate_limit: Arc<dyn SignupRateLimiter>,            // default: MemoryRateLimiter
}
```

Builder shape:

```rust
AuthState::new(users, sessions, tokens)
    .with_signup_open(Role::Reader)
    // or
    .with_signup_invite(Arc::new(SqliteInviteStore::new(pool)))
```

When `signup == Disabled`, neither `POST /auth/signup` nor the
admin invite endpoints are mounted. When `signup == Invite`, the
admin invite endpoints (`POST /auth/invites`,
`GET /auth/invites`, `DELETE /auth/invites/{code}`) are mounted
and protected by `require_role(Admin)` exactly the way
`starter-server` already protects admin-scoped routes.

### R10 — Comments explain *why*, never *what*; doc-comments on every public item

Same convention as the rest of the workspace. No banners, no emoji,
no progress logs. The handler's inline comments justify the
deliberately surprising bits (the atomic invite-consume UPDATE; the
non-leaky 409; the pre-hash rate-limit check) and nothing else.

## Repo layout (additions to `crates/starter-auth-users/`)

```
crates/starter-auth-users/
  migrations/
    0003_invites.sql                  <- starter_auth_users_invites (sqlite + pg)
    0004_users_email_verified.sql     <- ALTER ADD COLUMN email_verified
  src/
    routes/
      signup.rs                       <- POST /auth/signup
      invites.rs                      <- POST/GET/DELETE /auth/invites (admin)
      router.rs                       <- conditionally mount above per SignupMode
      state.rs                        <- + signup, invites, rate_limit fields
    signup/
      mod.rs                          <- re-exports
      mode.rs                         <- SignupMode enum + env parser
      validate.rs                     <- email/password validation (shared w/ admin)
      blocklist.rs                    <- compile-time top-100 password set
      rate_limit/
        mod.rs                        <- SignupRateLimiter trait
        memory.rs                     <- default token-bucket impl
    store/
      invite_store.rs                 <- trait + sqlite impl (feature-gated)
  tests/
    signup_open_mode.rs
    signup_invite_mode.rs
    signup_disabled_returns_404.rs
    signup_rate_limit.rs
    signup_email_already_registered.rs
    signup_weak_password.rs
    signup_invite_single_use_race.rs
```

## Data model additions

```sql
-- 0003_invites.sql  (sqlite)
CREATE TABLE starter_auth_users_invites (
  code         TEXT PRIMARY KEY,
  email        TEXT NOT NULL,
  role         TEXT NOT NULL,
  issued_by    TEXT NOT NULL
                 REFERENCES starter_auth_users_users(id) ON DELETE CASCADE,
  issued_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_at   TEXT NOT NULL,
  consumed_at  TEXT,
  consumed_by  TEXT
                 REFERENCES starter_auth_users_users(id) ON DELETE SET NULL
);
CREATE INDEX idx_invites_email ON starter_auth_users_invites (email);

-- 0004_users_email_verified.sql  (sqlite)
ALTER TABLE starter_auth_users_users
  ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0;

-- 0004_users_email_verified.sql  (postgres)
ALTER TABLE starter_auth_users_users
  ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT FALSE;
```

The `email_verified` column is added unconditionally because every
user (admin-created, OAuth-created, signup-created) benefits from
having the slot; admin and OAuth flows set it to `true` on insert
(the operator vouches for admin emails; OAuth's verified-email
check vouches for those), signup sets it to `false`.

## Routes

| Method | Path                          | Purpose                                                | Auth                                |
| ------ | ----------------------------- | ------------------------------------------------------ | ----------------------------------- |
| POST   | `/auth/signup`                | Create user + mint session                             | none (rate-limited; mode-gated)     |
| POST   | `/auth/invites`               | Issue an invite code for a given email + role          | session + CSRF + `require_role(Admin)` |
| GET    | `/auth/invites`               | List invites (filterable by email, consumed-or-not)    | session + CSRF + `require_role(Admin)` |
| DELETE | `/auth/invites/{code}`        | Revoke an unconsumed invite                            | session + CSRF + `require_role(Admin)` |

Request / response shapes:

```jsonc
// POST /auth/signup  (open mode)
// → 200, body: { "csrf_token": "..." }, Set-Cookie: sas_… + starter_csrf
{ "email": "alice@example.com", "password": "..." }

// POST /auth/signup  (invite mode)
{ "email": "alice@example.com", "password": "...", "invite_code": "01J…" }

// POST /auth/invites  (admin)
// → 201, body: { "code": "01J…", "expires_at": "2026-05-27T…" }
{ "email": "bob@example.com", "role": "Reader", "ttl_days": 7 }
```

## Configuration

```
SIGNUP_MODE=disabled                # disabled | open | invite
SIGNUP_DEFAULT_ROLE=Reader          # open mode only
SIGNUP_PASSWORD_MIN_LEN=12
SIGNUP_RATE_LIMIT=memory            # memory | disabled
SIGNUP_INVITE_DEFAULT_TTL_DAYS=7    # invite mode only
```

`SIGNUP_MODE` is the only required setting; everything else has a
defensible default. The crate refuses to start (`AuthState::new`
returns an error) if `SIGNUP_MODE=invite` is set but no
`InviteStore` is wired, rather than silently downgrading.

## Flow (signup handler, invite mode)

1. Rate-limit check (per-IP + per-normalised-email). 429 on failure
   — **before** any DB work.
2. Validate email format and password policy. 400 on failure.
3. Look up the invite by `code`. If absent / expired / for a
   different email → `401 invite_required` (uniform; see R4).
4. Hash the password (Argon2id, the same params `admin::create_admin`
   uses today).
5. Generate the new user id (ulid), `UserStore::create(id, email,
   Some(&hash), role_from_invite)`. On `Conflict` → `409
   email_already_registered`.
6. Atomic invite-consume UPDATE (see R3). On `rows_affected == 0`
   → `409 invite_already_consumed` *and* delete the just-inserted
   user (best-effort; the user has not received a session yet,
   so the row is unreachable). The race window is narrow (between
   step 5 and step 6) and the cleanup keeps the table tidy; even
   if cleanup fails, the orphaned user has no session and no token
   and can be reaped by an admin.
7. Mint session via `SessionStore::create`; set cookies; return
   `200` with the CSRF token.

Open-mode flow is identical with steps 3 and 6 omitted and the role
sourced from `SIGNUP_DEFAULT_ROLE`.

## Testing seams

- `starter_auth_users::signup::rate_limit::NoRateLimit` — opt-in
  via `SIGNUP_RATE_LIMIT=disabled` or wired directly in tests.
  Every signup test uses it.
- `starter_auth_users::testing::test_app_with_signup(mode)` —
  factory returning a `TestApp` with an in-memory `UserStore`,
  `SessionStore`, optional in-memory `InviteStore`, and
  `NoRateLimit`. Drop-in for HTTP-level tests.
- `MemoryInviteStore` — in-process `HashMap<code, Invite>` impl
  for tests; the production code path is sqlite-only.

## Smoke tests (before merging)

### "Disabled mode returns 404, not 403" test

`SIGNUP_MODE=disabled` (the default). `POST /auth/signup` returns
`404`. `POST /auth/invites` returns `404`. OpenAPI does not list
either route. If they 403 instead, R2 has slipped and the surface
is leaking the feature flag.

### "Email enumeration via signup is closed" test

Create user `alice@example.com` (password-based). Signup as
`alice@example.com` returns `409 email_already_registered`. Create
`bob@example.com` (OAuth-only, no password). Signup as
`bob@example.com` also returns `409 email_already_registered` —
**identical body**. Signup as a brand-new email succeeds. If the
two existing-email branches return distinguishable bodies, R4 has
slipped.

### "Invite single-use under concurrent race" test

Issue one invite. Two clients call `POST /auth/signup` with the
same code simultaneously (tokio `join!` of two requests against
the same `TestApp`). Exactly one succeeds with a session; the
other receives `409 invite_already_consumed`. The `users` table
has exactly one new row. If both succeed, R3's atomic UPDATE has
slipped.

### "Invite for a@x cannot sign up b@x" test

Issue invite for `alice@example.com`. Signup with the right code
but `email = "bob@example.com"`. Response is `401 invite_required`
(R4 uniform error). No user created. If a user is created or a
granular error is returned, R3 or R4 has slipped.

### "Rate-limit gates before password hashing" test

`SIGNUP_RATE_LIMIT=memory`, default budget. Fire 6 signups with
unique emails from the same IP within 1 second. The 6th returns
`429` with `Retry-After`. Total wall-clock time of all 6 requests
is **below** the wall-clock time of 5 successful Argon2id hashes
on the test machine (asserted via a Criterion-style coarse
measurement, not a microbenchmark). If the 6th request takes ≥1
Argon2id-worth of CPU, R6 has slipped — the check ran *after*
hashing.

### "Signup mints the same session login does" test

Signup as a new user, capture the session cookie, and immediately
call `GET /auth/me`. Returns the principal. Log out via
`POST /auth/logout`, then log in with the same credentials via
`POST /auth/login`, and call `GET /auth/me` again. Both responses
have identical principal shape and identical cookie format
(name, attributes). If signup mints a cookie the rest of the
stack does not recognise, R1 has slipped.

### "Weak passwords are refused with a clear error" test

Signup with `password = "password1234"` (in the top-100 blocklist).
Returns `400 weak_password`. Signup with `password = "aaaaaaaaaaaa"`
(12 chars, meets length, not in blocklist). Returns `200` —
the policy is intentionally minimal; the SPA is expected to do
zxcvbn-grade scoring client-side. If the second one is refused,
R5 has slipped into "too strict" and consumer SPAs will reject
it as a moving target.

### "Admin can issue, list, and revoke invites" test

As admin, `POST /auth/invites { email: c@x, role: Writer }` →
201 with code + expires_at. `GET /auth/invites?email=c@x` returns
the row, `consumed_at: null`. `DELETE /auth/invites/{code}` →
204. Signup attempt with the revoked code → `401 invite_required`.

### "Signup-created user has `email_verified = false`" test

Signup, then read the user row directly via `UserStore::find_by_id`.
`email_verified` is `false`. Create the same shape via the admin
CLI; `email_verified` is `true`. If signup users come out
verified, R7 has slipped and Phase 2's verification flow becomes
a no-op for everyone who signed up before it shipped.

## Non-goals

- **Not email verification.** R7. Lands as a separate phase with
  its own SMTP / transport dependency.
- **Not password reset / forgot-password.** Same reason as
  verification: requires outbound mail. Separate scope doc when a
  consumer needs it.
- **Not MFA / TOTP / WebAuthn enrolment.** Signup mints the same
  session login does; any step-up factors are layered after that,
  not woven into the signup flow.
- **Not CAPTCHA.** The rate-limit seam plus operator-facing WAF
  guidance is the v0.1 answer. A consumer who needs hCaptcha /
  Turnstile wires a middleware in front of `/auth/signup` in their
  own router; the crate exposes the route shape it needs to
  bracket but does not bundle a CAPTCHA dependency.
- **Not third-party signup (OAuth).** That is `starter-auth-oauth`'s
  callback path. This scope is for *local* email+password signup.
- **Not username-based signup.** Email is the primary identifier in
  `starter_auth_users` and stays so. Display name is a separate,
  user-editable field; allowing handle-based login is a different
  schema decision and out of scope.
- **Not bulk invite import / CSV upload.** `POST /auth/invites` is
  one-at-a-time. An operator who wants to invite 500 users runs a
  script that calls it 500 times.
- **Not invite "links" with embedded base URLs.** The route returns
  the code; the operator (or their mail template) wraps it in
  whatever URL their SPA expects (`https://app.example.com/signup?invite=…`).
  The crate does not know what the consumer's frontend URL is.

## Decisions made

- **Signup is in `starter-auth-users`, not a new crate.** The
  surface is small (one user-facing route + three admin routes),
  the deps are already present, and splitting it would create a
  `starter-auth-signup` crate that must depend on `starter-auth-users`
  for the `UserStore` and be depended on by `starter-server` —
  more graph edges for no isolation benefit.
- **Disabled is the default.** R2. Backwards-compatible with
  every existing consumer; nobody gets a signup endpoint by
  upgrading the crate without changing config.
- **Invite codes are bound to email at issue.** R3. The alternative
  ("invite is a capability anyone with the link can redeem") makes
  invites equivalent to bearer tokens for account creation, and
  the leakage radius is too large.
- **Atomic UPDATE for invite consumption.** R3. The only correct
  shape across sqlite and postgres without requiring distributed
  transactions.
- **409 is uniform for email collision; no oracle.** R4. Login's
  `password_not_set` response is the place that legitimately
  discloses "OAuth user," and it does so behind the password gate.
- **Password policy is hardcoded minimal.** R5. zxcvbn-class
  policy is a frontend concern; server enforces the floor.
- **Rate limit pre-hash.** R6. Argon2id-after-rate-check is a
  one-line ordering decision that determines whether the route is a
  DoS vector.
- **Session minted on signup.** R1. The two-step signup-then-login
  alternative is uniformly worse for SPAs.
- **`email_verified` column ships now, gated by nothing.** R7.
  Cheap, future-proof, and matches the OAuth crate's existing
  verified signal.
- **Role is hard-coded (open) or invite-baked (invite); never
  client-supplied.** R8.
- **Admin invite endpoints reuse `require_role(Admin)`.** R9. No
  new authorization primitive.

## Open questions

- **Should `SIGNUP_MODE=invite` allow a self-service "request
  invite" endpoint?** A `POST /auth/invites/request { email }` that
  records a row in `starter_auth_users_invite_requests` for an admin
  to triage. Useful for closed-beta products. Defer until a
  consumer asks; the workaround today is a contact form on the
  marketing site.
- **Should invite codes be displayed in plaintext after issue, or
  only at issue time (hash-at-rest)?** Plaintext-after-issue is
  easier for admins to copy-paste twice; hash-at-rest follows the
  same pattern as `starter-auth-users` API tokens. Lean
  hash-at-rest, but the admin-facing UX cost is real and the
  decision should be made with the consumer who first ships invite
  mode at scale.
- **Audit-log event for new-user signup.** Same shape as the OAuth
  scope's open question: structured `tracing` event at `info` now,
  durable audit row when `starter-observability` has an audit-sink
  concept.
- **Should the signup handler accept a `display_name` field?** The
  user has to set one eventually. Lean yes (one optional field;
  default to the email local-part if absent); needs a small validator
  (length, no control chars). Easy to add without breaking the v0.1
  shape because serde tolerates extra fields by default.
- **What to do if `SIGNUP_MODE=open` is enabled in a deployment
  that *also* has OAuth wired with `OAUTH_SIGNUP_ENABLED=false`?**
  Today these are independent flags. Probably correct (a consumer
  may want "anyone can sign up with email+password, but only
  pre-existing users can link OAuth") but worth a CHANGELOG note
  to call out the interaction.

## Phasing

Each phase is independently mergeable.

### Phase 1 — Open-mode signup + the conditional router

- `SignupMode` enum, env parser, `AuthState` field.
- `routes/signup.rs` (open mode only).
- `validate.rs` + `blocklist.rs` (shared with admin-create — the
  admin CLI gains a call to the same validators in this phase).
- `MemoryRateLimiter` + `NoRateLimit`.
- `0004_users_email_verified.sql` migration (column lands now;
  invite table waits for Phase 2).
- Conditional mount in `auth_router`.
- Smoke tests: disabled-returns-404, mints-same-session,
  email-enumeration-closed, rate-limit-pre-hash, weak-password,
  email_verified-false.

Outcome: a consumer can flip `SIGNUP_MODE=open` and ship local
self-service signup, with sessions interchangeable with login's.

### Phase 2 — Invite mode + admin invite endpoints

- `0003_invites.sql` migration.
- `InviteStore` trait + sqlite impl + `MemoryInviteStore` for
  tests.
- `routes/invites.rs` (POST, GET, DELETE; `require_role(Admin)`).
- Signup handler grows the invite branch (atomic UPDATE).
- Smoke tests: invite-single-use-race, invite-bound-to-email,
  admin-issue-list-revoke.

Outcome: closed-beta products can ship.

### Phase 3 (reserved) — Email verification

- `0005_email_verification_tokens.sql` migration.
- `MailTransport` trait + `SmtpMailTransport` (feature-gated).
- `POST /auth/verify-email/send` + `GET /auth/verify-email?token=…`.
- Optional `SIGNUP_REQUIRE_VERIFIED_EMAIL_FOR_LOGIN=true` flag that
  gates `POST /auth/login` on `email_verified = true` (default
  `false` to stay backwards-compatible).

Lands when a consumer needs verified email or password reset
(they share the mail-transport dependency).

### Phase 4 (reserved) — Password reset

- `POST /auth/reset/request { email }` → emails a one-shot token.
- `POST /auth/reset/confirm { token, new_password }` → rotates
  hash, invalidates all existing sessions and tokens for the user.

Reuses the mail transport from Phase 3.

## Bottom line

**One handler, one optional table, one nullable column, three env
vars. Disabled by default. Open mode is a five-line config change
for the consumer; invite mode is one extra wire-up. Signup ends in
the same `sas_*` session login mints, so every downstream auth
check is unchanged. The non-leaky 409, the atomic invite consume,
and the pre-hash rate-limit are the three load-bearing rules that
keep the only writable un-authenticated route from being an abuse
vector.**
