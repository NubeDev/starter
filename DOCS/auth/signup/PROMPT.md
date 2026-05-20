# Signup — Phase 1 implementation prompt

Drop this into a fresh agent session to start coding Phase 1 of the
signup design.

---

Implement **Phase 1** of the signup design specified in
[DOCS/auth/signup/SCOPE.md](./SCOPE.md). Read that scope doc
end-to-end before writing code — every rule R1–R10 is load-bearing.

**Scope of this task (Phase 1 only):**
- Open-mode email+password signup in `starter-auth-users`. No invite
  table, no admin invite endpoints, no email verification flow.
- The `email_verified` column lands now (migration
  `0004_users_email_verified.sql`, sqlite + postgres variants).
  Admin-created and OAuth-created users get `true`; signup users get
  `false`. No route gates on it yet.

**Deliverables:**
1. **`SignupMode` enum** (`Disabled` | `Open { default_role: Role }`)
   + env parser, in `crates/starter-auth-users/src/signup/mode.rs`.
2. **`AuthState` gains** `signup: SignupMode`,
   `rate_limit: Arc<dyn SignupRateLimiter>` — defaults preserve
   existing behaviour (`Disabled`, `MemoryRateLimiter`). Builder
   methods `with_signup_open(Role)` and `with_rate_limiter(...)`.
   Do not break the existing `AuthState::new` signature.
3. **`POST /auth/signup` handler** in
   `crates/starter-auth-users/src/routes/signup.rs`. Flow per the
   scope's "Flow" section, open-mode subset: rate-limit → validate →
   hash → `UserStore::create` → mint session via the same
   `session::issue` path login uses. Reuse the cookie-setting code
   from `routes/login.rs` (extract a shared helper if it makes the
   duplication obvious; do not refactor login itself otherwise).
4. **Validation module** (`signup/validate.rs`): email parse + length
   ≤ 254; password length ≥ `SIGNUP_PASSWORD_MIN_LEN` (default 12)
   and ≤ 4096; not equal to email local-part; not in compile-time
   top-100 blocklist (`signup/blocklist.rs`,
   `OnceCell<HashSet<&'static str>>` baked from a vendored text file
   in the crate). Export the validator so `admin::create_admin`
   calls the same function — wire that call site in this PR.
5. **`SignupRateLimiter` trait** + `MemoryRateLimiter` (token bucket
   per-IP and per-normalised-email, 5 / 10 min each, whichever is
   tighter sets `Retry-After`) + `NoRateLimit`. Check **before** any
   DB work and **before** password hashing (R6 — ordering is
   load-bearing).
6. **Conditional mount** in `routes/router.rs`: `signup` route is
   mounted iff `state.signup != Disabled`. When disabled the route
   404s and does **not** appear in OpenAPI.
7. **OpenAPI**: add the new route + request/response schemas to
   `crates/starter-auth-users/src/openapi.rs`, conditional on mount.

**Response shapes** (match scope doc exactly):
- 200 → `{ "csrf_token": "..." }` + `Set-Cookie: sas_…` +
  `Set-Cookie: starter_csrf=…`.
- 400 → `{"error": "invalid_email" | "weak_password",
  "message": "..."}`.
- 409 → `{"error": "email_already_registered"}` — **uniform body
  regardless of whether the existing account is password or
  OAuth**. Do not include any field that distinguishes them (R4).
- 429 → `{"error": "rate_limited"}` + `Retry-After: <secs>` header.

**Tests (all must pass before you stop):**
- `tests/signup_disabled_returns_404.rs` — default config: route
  404s, OpenAPI does not list it.
- `tests/signup_open_mode.rs` — happy path: signup → session cookie
  → `GET /auth/me` returns the principal; same cookie shape as
  login's.
- `tests/signup_email_already_registered.rs` — collision with a
  password user and collision with an OAuth-only user (use
  `UserStore::create(..., None, ...)` to simulate the latter)
  return byte-identical 409 bodies.
- `tests/signup_weak_password.rs` — `"password1234"` (blocklist) →
  400; `"aaaaaaaaaaaa"` (12 chars, not in list) → 200.
- `tests/signup_rate_limit.rs` — 6 requests / same IP / unique
  emails → 6th is 429 with `Retry-After`; total wall-clock of all 6
  is less than 5× a single signup's hashing time (coarse assertion:
  measure one successful signup, multiply by 5, assert the
  6-request sequence finished faster than that). This is the proof
  R6 ordering is correct.
- `tests/signup_email_verified_false.rs` — signup user has
  `email_verified = false`; admin-created user has
  `email_verified = true`.

Use the existing `MemoryUserStore` / `MemorySessionStore` testing
helpers if they exist; if not, mirror the pattern from the login
tests in `crates/starter-auth-users/tests`.

**Constraints:**
- No new heavy dependencies. The `email_address` crate is fine if
  not already present; check `Cargo.toml` first and reuse what's
  there if possible.
- Doc-comments on every public item. Inline comments only for the
  deliberately surprising bits (the pre-hash rate-limit ordering,
  the uniform 409, the `email_verified=false` default). No banners,
  no emoji.
- Do not touch `starter-auth-oauth`. Do not touch the
  `Authenticator` trait. Do not introduce a new cookie or session
  model.
- Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test -p starter-auth-users` before declaring done.

When finished, give a one-paragraph summary: which files were
added/changed, which tests were added, and which scope rules each
new test maps to (R1–R10). Flag anything in the scope you couldn't
honour and why.
