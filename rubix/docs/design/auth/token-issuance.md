# `POST /auth/token` — credentials → bearer issuance

> Cites: SCOPE [R3](../../SCOPE.md#r3) (identity carve-out),
> mobile [THIN-SLICE Pre-Block 4](../../scope/mobile/THIN-SLICE.md#pre-block-4--backend-bearer-token-endpoint-blocker),
> [ADR 0004](../../adr/0004-react-native-mobile-app.md).

## What it is

Non-cookie bearer issuance for clients that can't carry cookies —
React Native, native desktop, CLI sign-in. Mirrors
`POST /auth/login` but returns a `sak_…` token instead of setting a
session cookie + CSRF cookie pair.

## Why it's separate from `/auth/login`

`/auth/login` is **cookie-shaped**. It returns `{ csrf_token }` and
relies on `Set-Cookie` headers to seed the SPA's session. Mobile
clients (and any non-browser caller) cannot consume that contract:

- RN's `fetch` has no cookie jar.
- `expo-secure-store` is the right home for a long-lived bearer; a
  cookie store would be the wrong fit even if it existed.
- The existing bearer **acceptance** path
  ([`principal_layer.rs`](../../../../crates/starter-auth-users/src/principal_layer.rs))
  is already operating on `Authorization: Bearer sak_…`. The gap
  was precisely the issuance counterpart.

Layering an "if you don't have cookies, also return a bearer" mode
onto `/auth/login` was rejected: it conflates two response shapes
(cookies vs. body-only), and forces every cookie consumer to filter
a field that doesn't apply to them.

## Decision — location

**`starter-auth-users`.** Every consumer of the multi-user auth
crate gets the route for free. The mobile thin-slice docs name this
as the default proposal (see
[scope/mobile/README.md §Backend prerequisites](../../scope/mobile/README.md#backend-prerequisites));
the alternative of mounting it only in `rubix-agent` was rejected:

- It would diverge from the cookie path's home (also
  `starter-auth-users`) for no reason — both are
  "credentials → credential" exchanges over the same `UserStore`.
- It would forbid every other starter consumer (current and
  future) from offering bearer login without re-implementing the
  password check, the `password_not_set` envelope, the tenant
  resolution, and the token-mint contract.
- The single-owner `starter-auth-token` crate already serves the
  "one operator, no users table" case and is mutually exclusive
  with `starter-auth-users`. Mobile's target is the multi-user
  path, so the new route belongs alongside the rest of that
  surface.

## Decision — payload shape

### Request

```json
{
  "email":     "u@example.com",
  "password":  "…",
  "tenant_id": "t-abc"            // optional
}
```

- `email`, `password` — same semantics as `/auth/login`.
- `tenant_id` — optional. Resolution rules:
  1. If present, it must match a membership row for `user_id`.
     Mismatch → **403**.
  2. If absent and the auth state carries a `TenantStore` (wired
     via `AuthState::with_tenants(...)`), the route reads
     `memberships_for_user(user_id)` and:
     - exactly one membership → use it;
     - multiple memberships → **409** with the membership list so
       the client can re-POST with an explicit `tenant_id`;
     - no memberships and the user holds the global `Admin`
       role → use the super-admin sentinel `"*"` (matches the
       behaviour of cookie-session admin paths);
     - no memberships and not admin → **403**.
  3. If absent and **no** `TenantStore` is wired, the route
     fails closed: `tenant_id` becomes required and the response
     is **400** (`missing_tenant_id`).

### Response

```json
{
  "token":       "sak_<id>.<secret>",
  "expires_at":  "2026-06-24T12:34:56Z",
  "token_type":  "Bearer"
}
```

- `token` — the plaintext returned exactly once. The server stores
  only an argon2id hash of the secret half (see
  [`token::issue`](../../../../crates/starter-auth-users/src/token/issue.rs)).
- `expires_at` — RFC3339 UTC. Advisory in v1 — clients react to
  401, they don't pre-emptively refresh. See
  [APP-SHELL §Token expiry](../../scope/mobile/APP-SHELL.md#token-expiry).
- `token_type` — always `"Bearer"`. Future-proofs the field for the
  day refresh tokens land (deferred per
  [NON-GOALS.md](../../scope/mobile/NON-GOALS.md#technical)).

### TTL

Default **30 days** (`TOKEN_DEFAULT_TTL_DAYS`). Reasoning: phones
sit in pockets; a 24h TTL (the cookie-session default) would force
re-auth multiple times a week and there is no refresh-token in v1.
The cookie path keeps its 24h default — browsers refresh more
naturally and have alternative recovery paths.

### Scope set

Empty (`scopes = &[]`). The bearer carries the same authority as the
user's cookie session — no scope narrowing at issuance time. This
matches the existing `token::issue` tests and the
"one-token-per-connection, full operator access" model the mobile
app needs.

## Error envelope parity with `/auth/login`

| Condition | Status | Body |
|---|---|---|
| Bad credentials | `401` | empty |
| `password_hash IS NULL` (OAuth-only account) | `400` | `{ "error": "password_not_set", "providers": [...] }` (same shape login emits) |
| `tenant_id` mismatch with memberships | `403` | empty |
| Ambiguous tenant (multiple memberships, no explicit pick) | `409` | `{ "error": "tenant_required", "memberships": [{tenant_id, role}, ...] }` |
| Missing tenant + no tenants store wired | `400` | `{ "error": "missing_tenant_id" }` |
| No tenant resolvable for non-admin user | `403` | empty |
| Token store failure | `500` | empty (logged via `tracing::warn`) |

The `password_not_set` envelope is **the same type** the login
route returns ([`PasswordNotSetResponse`](../../../../crates/starter-auth-users/src/routes/login.rs))
so clients that already pattern-match on `error: "password_not_set"`
work for both routes unchanged.

## What this does **not** do

- No refresh-token mint. Tracked as
  [NON-GOAL](../../scope/mobile/NON-GOALS.md#technical); future
  work adds a sibling `POST /auth/token/refresh`.
- No MFA challenge. Tracked separately; will branch off this route
  by returning a `requires_mfa` envelope, leaving the success path
  unchanged.
- No rate limiting beyond what the surrounding middleware applies.
  The signup rate limiter is intentionally not reused — credentials
  → bearer is a re-auth path, not an account-creation path; the
  caller is presumably an existing user.

## Test contract

`crates/starter-auth-users/tests/http.rs` adds:

1. Happy path — single membership: 200, `expires_at` is in the
   future, the returned bearer authenticates `/whoami`.
2. Multiple memberships, no explicit `tenant_id` → 409 with the
   membership list.
3. Multiple memberships, explicit `tenant_id` → 200 for that
   tenant.
4. Bad password → 401.
5. OAuth-only account → 400 `password_not_set` (parity with login).
6. Tenants store unwired + no `tenant_id` → 400
   `missing_tenant_id`.

## Mobile call site

See [APP-SHELL Strategy](../../scope/mobile/APP-SHELL.md#strategy) —
the two-step `useLogin()` calls this route, stores
`{ token, expires_at }` in `expo-secure-store`, then installs the
bearer on the in-memory `StarterClient` via
`tokenStrategy.login(client, { kind: 'token', token })`.
