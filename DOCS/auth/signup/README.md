# Signup

Self-service email+password signup for `starter-auth-users`. One new
route (`POST /auth/signup`), an env-driven gate (`SIGNUP_MODE`), and a
session minted on success — identical to `POST /auth/login`'s `sas_*`
cookie.

For the full design rationale and hard rules see
[`SCOPE.md`](./SCOPE.md). For the Phase 1 implementation prompt see
[`PROMPT.md`](./PROMPT.md).

## Quick start

```bash
# Enable open signup (default role: reader)
export SIGNUP_MODE=open
export SIGNUP_DEFAULT_ROLE=reader   # optional, defaults to reader
```

With `SIGNUP_MODE=disabled` (the default) the route is not mounted and
returns 404.

## Route

| Method | Path            | Auth | Description |
|--------|-----------------|------|-------------|
| POST   | `/auth/signup`  | none | Create account + mint session |

### Request

```json
{ "email": "user@example.com", "password": "at-least-12-chars" }
```

### Responses

| Status | Body | Meaning |
|--------|------|---------|
| 200    | `{ "csrf_token": "…" }` + `Set-Cookie: sas_…` + `Set-Cookie: starter_csrf=…` | Signed up and logged in |
| 400    | `{ "error": "invalid_email" \| "weak_password", "message": "…" }` | Validation failure |
| 409    | `{ "error": "email_already_registered" }` | Email taken (uniform, no account-type leak) |
| 429    | `{ "error": "rate_limited" }` + `Retry-After` header | Rate limit hit |

## Configuration

| Env var | Default | Description |
|---------|---------|-------------|
| `SIGNUP_MODE` | `disabled` | `disabled` / `open` |
| `SIGNUP_DEFAULT_ROLE` | `reader` | Role assigned to new accounts in open mode |
| `SIGNUP_PASSWORD_MIN_LEN` | `12` | Minimum password length |
| `SIGNUP_RATE_LIMIT` | (enabled) | Set to `disabled` to skip in-process rate limiting |

## Validation (R5)

- Email: must contain `@`, non-empty local + domain with `.`, ≤ 254
  chars.
- Password: length ≥ `SIGNUP_PASSWORD_MIN_LEN`, ≤ 4096, not equal to
  email local-part, not in compile-time top-100 breached-password
  blocklist.
- Same validation is enforced by `admin::create_admin`.

## Rate limiting (R6)

`MemoryRateLimiter` — token bucket, 5 requests / 10 minutes per IP
and per normalised email. Checked **before** password hashing to
prevent Argon2id CPU-DoS.

Override with `AuthState::with_rate_limiter(Arc::new(NoRateLimit))` or
set `SIGNUP_RATE_LIMIT=disabled`.

## email_verified column (R7)

Migration `0004_users_email_verified.sql` adds
`email_verified INTEGER NOT NULL DEFAULT 1`. Admin-created and
OAuth-created users get `true` (the default); signup-created users are
set to `false` via `UserStore::set_email_verified` after insert. No
route gates on the flag yet — Phase 2 adds the verification flow.

## Crate layout (additions)

```
crates/starter-auth-users/
  src/
    signup/
      mod.rs             – re-exports
      mode.rs            – SignupMode enum + env parser
      validate.rs        – email/password validation
      blocklist.rs       – compile-time top-100 password set
      rate_limit/
        mod.rs           – SignupRateLimiter trait + NoRateLimit
        memory.rs        – MemoryRateLimiter (default)
    routes/
      signup.rs          – POST /auth/signup handler
      state.rs           – + signup, rate_limit fields
      router.rs          – conditional mount
  migrations/
    starter_auth_users/
      0004_users_email_verified.sql
```

## Tests

| Test file | Scope rules |
|-----------|-------------|
| `signup_disabled_returns_404` | R2, R9 |
| `signup_open_mode` | R1 |
| `signup_email_already_registered` | R4 |
| `signup_weak_password` | R5 |
| `signup_rate_limit` | R6 |
| `signup_email_verified_false` | R7 |
