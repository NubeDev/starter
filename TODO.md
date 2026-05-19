# starter — work remaining

Living checklist of what's needed to make the scaffold under `crates/` and
`packages/` match SCOPE.md and reach a state a real consumer can depend on.
Order within each phase is roughly the order to do it in; phases are
strictly ordered (a later phase assumes the earlier ones have landed).

Audit baseline: see the in-tree audit done 2026-05-19. Today the workspace
does not compile, no tests exist, and five crates SCOPE calls for are
absent. Status markers: `[ ]` not started · `[~]` partial / stub bodies
exist · `[x]` done.

---

## Phase 0 — Unblock the build

Nothing else can move until `cargo check --workspace` is green.

- [ ] Restore workspace `Cargo.toml` integrity. The reconstructed file
      omits the three `examples/*` members because those directories
      don't exist on disk; re-add them once the example crates land
      (Phase 5). Confirm `[workspace.dependencies]` matches the original
      (diff against any prior backup).
- [x] Fix utoipa-5 incompatibilities in `starter-spi`:
  - dropped the v4 `#[aliases]` attribute on `Page<T>` and gated the
    type parameter with `T: ToSchema` instead.
  - derived `ToSchema` on `Cursor` so `Page<Cursor>` fields compile.
  - also enabled the sqlx `migrate` feature on both store crates so
    `sqlx::migrate::Migrator` resolves.
  - mopped up clippy fallout: non-canonical `Clone` on `Id`, the
    `clippy::result_large_err` config error (boxed `figment::Error`),
    unused type-param on `health_router::<S>` / `metrics_router::<S>`,
    re-exported `CommandError` from `starter_cli::registry`.
- [x] Wire missing `@hugeicons/*` packages, or replace those icons.
      The deps were already declared in `packages/starter-ui-kit/
      package.json`; `pnpm -r typecheck` passes once node_modules
      exists. TODO baseline was stale on this one.
- [x] Add a `build` script to `packages/starter-ui-kit/package.json`.
      Mirrors `typecheck` for now since this package ships source
      directly (`main`/`types` point at `.ts`/`.tsx`).

Exit criteria: `cargo check --workspace`, `cargo clippy --workspace --
-D warnings`, `pnpm -r build`, `pnpm -r typecheck` all pass.

---

## Phase 1 — Make `starter-spi` shape-complete

`starter-spi` is the contracts crate; everything downstream is shaped by
it. Closing the gaps here is what unblocks Phases 2-4.

- [x] Move `Role` into `starter-spi` (`crates/starter-spi/src/auth/
      role.rs`). The old `starter_auth::role::Role` is now a
      `pub use` re-export so call sites in `starter-auth` keep
      compiling.
- [x] Move `Scope` into `starter-spi` (`crates/starter-spi/src/auth/
      scope.rs`), with the same re-export shim in `starter-auth`.
- [x] Add a `role` field to `Principal`. New shape:
      `Principal { subject, role, scopes: Vec<Scope>, extra }`.
- [x] Add `trait SecretStore` to `starter-spi` (`crates/starter-spi/
      src/secrets/`). Sync, with `ready() -> bool`, `get/put/delete`
      returning `Result<_, SecretError>`. `Secret` is a small newtype
      whose `Debug` redacts the value; zeroize policy is documented
      as "callers must not retain the source string".
- [x] Add `trait AiRunner` to `starter-spi` (`crates/starter-spi/src/
      ai/`). Associated types lifted verbatim: `Provider`,
      `RunnerInput::{Cli(CliCfg), Rest(RestCfg)}`, `Event`,
      `RunResult`, `RunnerError`. `Cancel` is a local trait so spi
      doesn't pull `tokio_util`; the concrete impl in `starter-ai`
      will wrap `CancellationToken`.
- [x] Decided `Authenticator` signature (SCOPE open question 3): stays
      `verify(&str)`. Rationale lives on the trait docs
      ([crates/starter-spi/src/auth/authenticator.rs](crates/starter-spi/src/auth/authenticator.rs)).
      `&http::request::Parts` and a richer `AuthContext` were both
      rejected — `&str` keeps the trait useful from every transport
      (HTTP, MCP, future gRPC), and the HTTP boundary already
      pre-parses bearer + session cookie into a single string inside
      `starter_server::auth::with_principal`.
- [x] Decided `SecretStore` sync vs async (SCOPE open question 5):
      stays **sync**. Rationale on the trait module docs
      ([crates/starter-spi/src/secrets/store.rs](crates/starter-spi/src/secrets/store.rs)).
      Both shipped impls (keyring, age-file) are sync at the bottom;
      secrets read at startup, not on the request hot path. Future
      network-backed impls should cache aggressively + use
      `block_in_place` on cold paths, OR ship a sibling
      `AsyncSecretStore`.
- [x] Round out paging primitives. Added `Sort { field, direction }`
      (`crates/starter-spi/src/sort/sort.rs`) with `Sort::asc/desc`
      helpers, and `Filter { predicates: Vec<Predicate> }`
      (`crates/starter-spi/src/filter/filter.rs`) with a chainable
      `Filter::and`. Both derive `ToSchema`.
- [x] Add `tests/compile.rs` real assertions. Now locks:
      reachability of every public re-export, `Principal` shape,
      `Secret` redaction, `Sort`/`Filter` builders, and object-safety
      smoke checks for `Authenticator`, `SecretStore`, `AiRunner`
      (5 tests, all passing).

Exit criteria: `starter-spi` exports `Role`, `Scope`, `SecretStore`,
`AiRunner` (+ associated types), `Principal { user_id, role, scopes }`;
no `starter-*` Cargo dep; compile test covers every public trait.

---

## Phase 2 — Storage actually stores

The store crates have the right *shape* but the migration runner is a
no-op and there's no actual SQL anywhere in the repo. Until this lands,
no auth crate can persist anything.

- [x] Implement the namespaced migration runner. API settled as the
      fluent chain (SCOPE open question 2): `migrate(pool)
      .with_source(s).with_source(s2).run().await`. Each source is
      applied into its own `_sqlx_migrations_<name>` table, version
      / description / installed_on / checksum, with a SHA-384 (sqlx's)
      checksum mismatch check on re-run. `MigrationSource` now holds
      `&'static sqlx::migrate::Migrator` — `Migrator: !Clone` in
      sqlx 0.8, so taking a borrow is the only ergonomic shape that
      keeps the `sqlx::migrate!(...)` macro working at the call site.
- [x] `connect(url)` for both stores. The postgres twin was already
      in place; the TODO baseline was stale.
- [x] Real cursor encoding: `base64url_nopad(version_byte ||
      json([sort, id]))`. `CURSOR_VERSION = 1`; `decode` returns
      `None` on unknown versions so the format can evolve. Identical
      bytes in both stores. 4 unit tests on the sqlite codec
      (round-trip, unicode + separators in payload, garbage, unknown
      version).
- [x] Settled `Repository<T>` derive scope (SCOPE open question 1):
      **deferred to v0.2**. Rationale on the paging module docs
      ([crates/starter-spi/src/paging/mod.rs](crates/starter-spi/src/paging/mod.rs))
      — no consumer to design against, hand-written sqlx queries are
      fine at current volume, and the macro carries permanent
      maintenance / debugging cost. Recommended scope when it does
      land: `find_by_id`, `list`, `insert`, `update` (optimistic
      version bump), `delete`. Nothing more.
- [x] Implemented `testing::with_database` in
      [crates/starter-store-postgres/src/testing/with_database.rs](crates/starter-store-postgres/src/testing/with_database.rs)
      using `testcontainers` 0.23 + `testcontainers-modules` 0.11
      (postgres module). Returns `(Pool, ContainerGuard)`; drop the
      guard last. SCOPE's "0.x vs 0.20+" question was a stale read —
      0.20+ never shipped; the coordinated current majors are
      0.23 / 0.11 and that's what we pin. Integration tests in
      [crates/starter-store-postgres/tests/migrate.rs](crates/starter-store-postgres/tests/migrate.rs)
      mirror the sqlite suite (`two_sources_apply_without_colliding`,
      `rerun_is_a_noop`); both marked `#[ignore]` since they need
      Docker. CI runs them via `cargo test -p starter-store-postgres
      --features testing -- --ignored` on every PR.
- [x] First migrations under `migrations/starter/0001_init.sql` in
      both store crates — a `starter_meta` key/value table. Real
      starter-owned tables land with the auth crates; this just
      gives the namespaced runner something to apply on first boot.
- [x] Integration tests against a real in-memory SQLite. 3 tests in
      `crates/starter-store-sqlite/tests/migrate.rs`: two sources at
      version 1 apply without colliding, re-run is a no-op, invalid
      source names rejected. The matching Postgres integration test
      lands with `testing::with_database`.

Exit criteria: `migrate(pool).with_source("starter", ...)
.with_source("app", ...).run().await` works on both backends; each
source lands in its own table; integration tests prove no collision.

---

## Phase 3 — Auth: split, then implement one side

SCOPE 228–232 mandates two mutually-exclusive crates,
`starter-auth-token` (headless / single-owner) and `starter-auth-users`
(multi-user). The repo has a single `starter-auth` that's stubbed end to
end. Split first, then pick `starter-auth-token` to implement first
because it's smaller and unblocks the "headless appliance" smoke test
(SCOPE 735–741).

- [x] Split the existing `starter-auth` crate:
  - `crates/starter-auth-token/` — pending+claimed records, claim
    flow, bearer verification. Built from scratch (the lift from
    `NubeDev/token-service` was unnecessary — the contract is small
    enough that copying it from spec was clearer than porting).
  - `crates/starter-auth-users/` — receives the prior `starter-auth/`
    contents wholesale. Bodies still `todo!()`; this commit just
    renames the crate and updates the lib-doc so the split is real.
    The argon2 / sessions / login-handlers work is the next bite.
  - Old `starter-auth/` directory removed; `[workspace.dependencies]`
    updated to list the two new crates instead.
- [x] Update workspace `Cargo.toml` members and `[workspace.dependencies]`
      to list the two new crates. Also added `rand`, `sha2`, `subtle`
      to `[workspace.dependencies]` (used by the token claim flow).
- [x] Implement `starter-auth-token` end-to-end:
  - [x] First-boot claim flow: `regenerate_claim_pending(store)`
        generates 32 random bytes, base64url-no-pad encodes them, and
        stores them in `starter_auth_token_pending`. The `SecretStore`
        write at `auth-token:pending` is deferred to when a binary
        wires both crates together — the trait now exists in spi but
        no concrete impl ships yet (Phase 4).
  - [x] `POST /auth/claim` handler in
        [crates/starter-auth-token/src/routes/claim.rs](crates/starter-auth-token/src/routes/claim.rs).
        Consumes pending in a transaction, stores SHA-256 of
        issued `owner_token`, returns plaintext exactly once.
        409 on `AlreadyClaimed` / `NoPending`, 401 on
        `InvalidToken`, 500 on store error.
  - [x] `Authenticator` impl
        ([crates/starter-auth-token/src/authenticator.rs](crates/starter-auth-token/src/authenticator.rs)):
        SHA-256 the presented bearer, constant-time compare against
        the stored digest (`subtle::ConstantTimeEq`), return
        `Principal { subject: claim_id, role: Admin, scopes: vec![] }`.
  - [x] Three migrations under source `starter_auth_token`:
        `_pending`, `_claimed`, plus `_epoch` (single-row,
        `id = 1`, bumped on every reset) so cached bearers can be
        invalidated externally.
  - [x] `regenerate_claim_pending(store)` reset path that wipes
        claimed+pending and bumps the epoch in one transaction.
        CLI wiring (`starter-cli reset --force`) lands with Phase 5.
  - [x] 8 integration tests in
        [crates/starter-auth-token/tests/claim_flow.rs](crates/starter-auth-token/tests/claim_flow.rs)
        covering: first-boot flow, owner-token verify happy path,
        wrong-token reject, unclaimed reject, replay reject,
        invalid-pending reject, no-seed reject, factory-reset
        invalidates prior owner. All passing.
- [x] Implement `starter-auth-users` end-to-end. All eight `todo!()`
      bodies are gone; the crate now has working argon2id passwords,
      DB-backed sessions, API tokens, /auth routes, CSRF, and a
      bridging `Authenticator`. Key shape decisions made along the
      way:
  - [x] argon2id password hash + verify via `password-auth`
        ([crates/starter-auth-users/src/password/](crates/starter-auth-users/src/password/)).
  - [x] Sessions on our own thin DB-backed store, not
        `tower-sessions` + `axum-login`. The latter would have
        introduced a heavy framework (5+ extra crates, specific
        trait shapes that don't fit our pool abstraction) for
        ~100 lines of behaviour: opaque server-issued id in the
        cookie, table lookup on each request, revoke = update row.
        The trait seam in spi doesn't change, so swapping in those
        crates remains a consumer-driven option. Session prefix
        `sas_` mirrors the API-token prefix `sak_` so the
        authenticator routes by string prefix without an extra DB
        round-trip.
  - [x] API token issue / verify / revoke against
        `starter_auth_users_tokens`. Token format
        `sak_<public_id>.<secret>` — the public id is the cleartext
        table key (O(1) lookup); the secret half is argon2id-hashed
        at rest. Plaintext shown once on issue, never again.
  - [x] `/auth/login`, `/auth/logout`, `/auth/me` handlers wired in
        [crates/starter-auth-users/src/routes/](crates/starter-auth-users/src/routes/).
        Handlers close over `Arc<AuthState>` (set up via
        `AuthState::new`) rather than threading through axum's
        `State` extractor, so the auth router merges into any
        consumer `Router<S>` without state-type gymnastics.
  - [x] `AuthAuthenticator` dispatches by credential prefix
        (`sak_` → token path, `sas_` → session path, anything else
        → `Unauthenticated` without a DB hit).
  - [x] Three migrations under source `starter_auth_users` in
        [crates/starter-auth-users/migrations/starter_auth_users/](crates/starter-auth-users/migrations/starter_auth_users/):
        `0001_users.sql` (users table, email-unique), `0002_sessions.sql`
        (cookie sessions with paired CSRF token + expiry + revoked_at),
        `0003_tokens.sql` (hashed_token + JSON scopes + expires_at +
        revoked_at).
  - [x] CSRF on mutating cookie routes. `/auth/login` returns the CSRF
        token in the response body AND sets it on a non-httpOnly
        `starter_csrf` cookie. `/auth/logout` (the only mutating cookie
        endpoint shipped today) requires the `X-CSRF-Token` header to
        match the cookie — missing / mismatched → 403. Bearer-token
        routes skip CSRF (the Authorization header isn't auto-attached
        by browsers, so there's no cross-site forgery surface).
  - [ ] `starter-cli admin create --email --role admin` bootstrap.
        Deferred — see
        [crates/starter-cli/src/commands/admin_create.rs](crates/starter-cli/src/commands/admin_create.rs)
        for the template; the command itself needs the consumer's DB
        pool, and `starter-cli` is deliberately store-agnostic
        (SCOPE: talks to the server via `starter-client-rs`, never
        the DB). Consumers wire their own subcommand calling
        `starter_auth_users::admin::create_admin` against their pool.
        Reintroduce as a generic-over-store `AdminCreate<U:
        UserStore>` when a consumer wants a copy-paste-free fit.
  - [x] Integration tests for both credential paths
        ([crates/starter-auth-users/tests/flow.rs](crates/starter-auth-users/tests/flow.rs),
        [crates/starter-auth-users/tests/http.rs](crates/starter-auth-users/tests/http.rs)).
        6 lib-level tests (create admin → session → verify, conflict
        on duplicate email, revoked session does not verify, API
        token round-trip, tampered secret → invalid, authenticator
        prefix dispatch) plus 2 HTTP-level tests against a real
        `TestApp` (login → /whoami → CSRF-protected logout, reader
        blocked from admin route).
- [x] `require_role` / `require_scope` middleware factories moved to
      `starter-server` per SCOPE 458–460. Now live in
      [crates/starter-server/src/auth/](crates/starter-server/src/auth/)
      as `with_principal(router, authenticator)` →
      `with_role(router, role)` → `with_scope(router, scope)` router-
      extension helpers (same shape as the rest of starter-server's
      middleware; see Phase 5 note on why `Layer` factories were
      avoided). Generic over `S` so they apply to consumer
      `Router<S>`. **Layer order matters**: `with_principal` must be
      the outermost wrap so the principal extension is set before the
      guards read it — documented at the
      `starter_server::auth` module doc. `with_principal` reads
      `Authorization: Bearer …` first, falls back to the
      `starter_session` cookie. Missing credentials are not 401 at
      this layer — guards (or the route handler) decide.

Exit criteria — met: a binary picks `starter-auth-token` and runs
the claim → owner-token → bearer-auth flow with real persistence;
a different binary picks `starter-auth-users` and runs login / me /
logout + API tokens against a real listener (2 HTTP smoke tests
prove it end-to-end). The two are mutually exclusive via cargo
features in the consumer's `Cargo.toml`.

---

## Phase 4 — Secrets and AI

These two areas were redesigned after the original scaffold. Phase 4
landed 2026-05-19: three new crates, the `auth-token <-> SecretStore`
wire-up, the spi AI shape brought into line with the upstream lift.

- [x] Create `crates/starter-secrets-keyring/` (SCOPE 189–193, 556–563).
      Wraps the `keyring` crate at v3.6 with `apple-native`,
      `windows-native`, and `sync-secret-service` features (and
      `crypto-rust` so a default build doesn't pull libssl). Service
      name = the consumer's binary crate name passed to
      `KeyringSecretStore::new`; each entry's "user" field is
      `<binary>:<name>` so two starter-based apps on the same machine
      don't collide. `ready()` probes with a benign `get_password`
      against the platform service; `NoEntry` counts as ready, every
      other error means the backend can't serve (Linux without DBus,
      etc.) and consumers should feature-swap to file.
- [x] Create `crates/starter-secrets-file/` (SCOPE 195–201, 565–572).
      Single age-encrypted file under `$XDG_DATA_HOME/<binary>/
      secrets.age` (ASCII-armored, JSON object inside). Identity
      resolution order: `STARTER_SECRETS_KEY` env var, then the
      consumer's config path passed via
      `FileSecretStoreBuilder::identity_path`, then first-run generation
      (writes `identity.age-key` next to the secrets file and prints a
      one-time `tracing::warn!` with the public key + backup path).
      `parking_lot::Mutex` round-trip cache; atomic rename on write.
      4 unit tests in [crates/starter-secrets-file/src/store.rs](crates/starter-secrets-file/src/store.rs)
      (round-trip, delete, persists across instances, ready).
- [x] Add both to workspace members + `[workspace.dependencies]`.
      Also added `starter-ai` in the same pass.
- [x] Wire `starter-auth-token` to read/write `auth-token:pending`
      through `SecretStore` when one is supplied (SCOPE 488–492). Shape
      chosen: a new sibling function
      `regenerate_claim_pending_with_secrets(store, secrets)` rather
      than mutating the existing function's signature — both functions
      live in [crates/starter-auth-token/src/claim/regenerate.rs](crates/starter-auth-token/src/claim/regenerate.rs).
      The key constant `PENDING_SECRET_KEY = "auth-token:pending"` is
      re-exported at the crate root. New test
      `regenerate_with_secrets_writes_plaintext_to_store` exercises it
      end to end against an in-process `HashMap`-backed
      `SecretStore`.
- [x] Reshaped `starter-spi`'s `ai::*` to match the real upstream
      before the lift (the previous spi was a sketch that didn't match
      `codeless-workspace/ai-runner`). Concrete changes: `Event` is
      now `{ session_id, provider, kind: EventKind }` with the five
      upstream `EventKind` variants (`Connected`, `Text`, `ToolUse`,
      `Done`, `Error`); `CliCfg` and `RestCfg` carry the full upstream
      field set (history, tool defs, tool_choice, permission_mode,
      …); `RunResult` carries tokens, cost, tool-call log, etc.;
      `RunnerError` collapses to `WrongInputKind` only (upstream
      transport / network errors flow through `RunResult::error`);
      `OnEvent` is now `tokio::sync::mpsc::Sender<Event>` (spi gains
      a `tokio = { features = ["sync"] }` dep — channels are pure
      data structures, not a runtime pull). The `Cancel` trait grew
      a `cancelled<'a>(&'a self) -> Pin<Box<Future + 'a>>` method so
      the upstream's `tokio::select! { _ = cancel.cancelled() }`
      pattern transfers verbatim.
- [x] Create `crates/starter-ai/` as a clean lift from
      `codeless-workspace/ai-runner` (SCOPE 203–210, 582–631; q7
      picks "clean lift" — this crate is the source of truth from
      this point). Per-provider feature gates, all default-off; the
      registry's `with_defaults()` populates only providers whose
      feature is enabled at compile time. Files in `runners/` were
      copied verbatim from the upstream and adapted with a mechanical
      rewrite: `crate::runner::{Runner, OnEvent}` →
      `starter_spi::ai::{AiRunner, Cancel, OnEvent}`; `crate::types::`
      → `starter_spi::ai::`; `cancel: CancellationToken` →
      `cancel: &dyn Cancel`. `TokenCancel` (in
      [crates/starter-ai/src/cancel.rs](crates/starter-ai/src/cancel.rs))
      wraps `tokio_util::sync::CancellationToken` and implements
      spi's `Cancel`.
  - [x] `provider-claude` via `claude-wrapper` pinned `=0.5.1` (SCOPE
        820–824). Pin is intentional; canary CI repo deferred (see
        below).
  - [x] `provider-codex`, `provider-copilot` CLI wrappers — both via
        `tokio::process` only, no extra deps.
  - [x] `provider-anthropic` via `anthropic-ai-sdk =0.2.27` (matches
        upstream).
  - [x] `provider-openai` via `async-openai =0.35.0` with
        `default-features = false, features = ["chat-completion",
        "rustls"]` — 0.35's feature set splits the chat types from
        the API client, so both knobs are needed for the upstream
        runner's imports to resolve.
  - [x] `Registry::with_defaults()` populates every enabled provider
        behind `#[cfg(feature = "provider-*")]`.
  - [x] Secret integration: `api_key_for(secrets, provider)` in
        [crates/starter-ai/src/secret.rs](crates/starter-ai/src/secret.rs)
        checks `SecretStore::get("ai:<provider>:api_key")` first and
        falls back to `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`.
  - [x] Cancellation via `CancellationToken`. CLI runners already use
        `kill_on_drop(true)` upstream; REST runners select against
        `cancel.cancelled().await` thanks to the spi trait extension
        above.
  - [ ] Canary-CI repo for `claude-wrapper` stream-json drift (SCOPE
        q6). Deferred — recommendation captured in the crate's lib
        doc: a separate `starter-ai-canary` repo so a green canary
        does not gate normal CI. Provisioning happens before first
        release.
- [x] Add `starter-ai` to workspace + `[workspace.dependencies]`.

Exit criteria — the "headless appliance" smoke test (SCOPE 735–741)
passes. Verified 2026-05-19 by `cargo tree` against a synthetic
package depending on `starter-auth-token` (feature `sqlite`) +
`starter-secrets-file` + `starter-ai` (feature `provider-anthropic`):
none of `starter-auth-users`, `starter-secrets-keyring`,
`claude-wrapper`, `async-openai`, codex/copilot deps appear in the
resolved tree.

---

## Phase 5 — Server, CLI, MCP: finish the wiring

Wiring phase. The crates already existed shape-first; this round
filled the bodies so a binary can stand up a real server with the
starter routes mounted, scrape Prometheus metrics, run MCP tools over
stdio, and hit the same server with the built-in `starter-cli`
subcommands. Examples are split off — they need either the full
auth-users body (Phase 3 tail) or Phase 6 TS work, so they land
together later.

- [x] Wire CORS + tracing middleware in
      [crates/starter-server/src/builder/server_builder.rs](crates/starter-server/src/builder/server_builder.rs).
      Mount order: consumer routers merged → starter routes
      (`/health`, `/metrics`, `/openapi.json`) → `with_state` → outer
      middleware (request-id, CORS, `tower_http::trace::TraceLayer`,
      latency). Defaults: `CorsLayer::very_permissive()` (override via
      `with_cors`), tracing layer always on. `with_metrics(registry,
      metrics)` is the gating call — `/metrics` and the latency
      middleware only mount when a `prometheus::Registry` plus a
      `StandardMetrics` handle are provided.
- [x] `init_tracing` now returns a `TracingGuard`. The guard is a
      `#[must_use]` zero-cost wrapper today (subscriber writes
      straight to stdout) — the type exists so call sites already use
      `let _guard = init(...)?;` and a future file-appender / OTLP
      exporter layer can be added behind it without touching every
      `main()`.
- [x] Implement `StandardMetrics::register`
      ([crates/starter-observability/src/metrics/standard.rs](crates/starter-observability/src/metrics/standard.rs)).
      Three metrics: `starter_requests_total{method,path,status}`,
      `starter_request_duration_seconds{…}` with a 12-bucket
      geometric-ish ladder from 1 ms to 10 s, and
      `starter_requests_in_flight`. 2 unit tests
      (registers clean, double-register is an error).
- [x] Implement `request_id_layer` + `latency_layer` — though as
      `with_request_id(router)` / `with_latency(router, metrics)`
      router-extension helpers rather than `Layer` factories.
      Reason: `axum::middleware::from_fn` returns a closure-typed
      layer whose exact shape can't be spelled without TAIT, and
      a one-call-site helper is cleaner than papering over that with
      `Box<dyn Layer>`. Lives in
      [crates/starter-server/src/middleware/](crates/starter-server/src/middleware/);
      observability keeps only the data types (`RequestId`,
      `REQUEST_ID_HEADER`, `StandardMetrics`). Observability lost its
      `tower` and `http` deps in the process — it's now `tracing` +
      `prometheus` + `uuid` only.
- [x] `/metrics` route encodes the registry as Prometheus text
      ([crates/starter-server/src/routes/metrics.rs](crates/starter-server/src/routes/metrics.rs)).
      The handler closes over `Arc<Registry>` rather than threading
      it through axum state, so the route can be merged into any
      consumer `Router<S>` without state coercions.
- [x] 3 integration tests in
      [crates/starter-server/tests/server_smoke.rs](crates/starter-server/tests/server_smoke.rs)
      against a real `TestApp`: `/health` returns 200 with the
      `X-Request-Id` header echoed verbatim; `/metrics` body contains
      `starter_requests_total` + `path="/health"` after one request;
      `/openapi.json` serves the consumer's doc.
- [x] Implement `starter-mcp` dispatch
      ([crates/starter-mcp/src/server/dispatch.rs](crates/starter-mcp/src/server/dispatch.rs)).
      Methods: `initialize` (returns `serverInfo` + `tools` capability,
      `protocolVersion = "2024-11-05"`), `ping`, `tools/list`
      (enumerates registry definitions), `tools/call` (looks up by
      name, invokes, wraps the JSON result in MCP's `content` +
      `structuredContent` envelope). Unknown methods → `-32601`,
      missing `name` → `-32602`, tool errors → `-32603`, malformed JSON
      → invalid_params with `id: null`. 7 unit tests cover all the
      above paths.
- [x] MCP HTTP transport ([crates/starter-mcp/src/server/http.rs](crates/starter-mcp/src/server/http.rs))
      behind `feature = "http"`. `mcp_router(registry, opts)` returns
      an `axum::Router<S>` exposing `POST /mcp` — single JSON-RPC
      envelope in, JSON response out, `204 No Content` for
      notifications. Optional `McpHttpOptions::with_auth(authenticator)`
      enforces `Authorization: Bearer …` via the spi `Authenticator`
      trait — same trait used by `starter_server::auth::with_principal`
      so `TokenAuthenticator` / `AuthAuthenticator` work unchanged.
      On 401 the body is a JSON-RPC error frame (`code: -32001`) so
      MCP clients see a structured error rather than an opaque HTTP
      status. 5 integration tests (open route, notification → 204,
      missing/invalid/valid bearer paths). Wired into
      [examples/minimal](examples/minimal/src/server.rs) — same
      `TokenAuthenticator` instance guards both `/hello` and `/mcp`,
      with an `echo` tool registered for the e2e test. SSE (Streamable
      HTTP) progress events deferred to v0.2 as a `mcp_sse_router`
      sibling; `Tool::invoke` stays single-shot.
- [x] Flesh out `starter-cli` commands. `health` and `openapi` both
      take `--base-url` (defaults `http://localhost:8080`, also reads
      `STARTER_BASE_URL`), build a `starter_client_rs::Client`, and
      pretty-print the JSON body. `CommandRegistry` gained
      `register_starter_defaults`, `subcommands()` (for binary main
      to attach to its root clap), and `dispatch` (looks up by parsed
      subcommand, runs it). 3 integration tests in
      [crates/starter-cli/tests/dispatch.rs](crates/starter-cli/tests/dispatch.rs)
      against a real `TestApp`: `health` and `openapi` round-trip,
      bare invocation yields `UserFacing("no subcommand given")`.
      `serve` and `migrate` deferred — they run inside the consumer's
      binary process (not against a remote server through the client),
      so they need access to the consumer's `AppState` / `Pool`.
      Lands when `examples/minimal/` does (so there's a concrete shape
      to design against).
- [x] `admin create` bootstrap — settled as **consumer-owned**, not a
      starter-cli built-in. Reason: it needs the consumer's `Pool`
      and pulling `starter-auth-users` into `starter-cli` would
      cross the store-agnostic boundary (SCOPE R8). The template at
      [crates/starter-cli/src/commands/admin_create.rs](crates/starter-cli/src/commands/admin_create.rs)
      is the canonical copy-paste for consumers wiring their own
      `admin` subcommand (call `starter_auth_users::admin::create_admin`
      against the consumer's pool).
- [x] [examples/minimal/](examples/minimal/) — server + sqlite + cli
      + auth-token in one binary. Subcommands: `serve`, `migrate`,
      `claim-reset` (consumer-owned local commands) + `health`,
      `openapi` (via `register_starter_defaults`). E2E integration
      test in [examples/minimal/tests/e2e.rs](examples/minimal/tests/e2e.rs)
      exercises the full claim → bearer → `/hello` round-trip
      against a real listener. Doubles as the canonical layout
      pattern for the **serve/migrate placement decision** —
      local-state commands live in the consumer binary, not in
      `starter-cli`. Added as a workspace member.
- [ ] `examples/full/` — server + postgres + mcp + react admin +
      docker. Deferred — blocks on a real consumer to design against;
      `examples/minimal/` already covers server + sqlite + auth +
      MCP-over-HTTP, which is the harder integration. The remaining
      delta is "swap sqlite → postgres + ship a React admin" — both
      are mechanical once a consumer asks for it.
- [x] [examples/gh-report/](examples/gh-report/) — skeleton CLI
      built on `starter-cli` + `starter-observability`. The `report`
      subcommand prints a stubbed JSON body so the consumer-domain
      layout is locked while the actual GitHub-API integration stays
      consumer-owned (drop in `octocrab`, fetch the PAT via
      `SecretStore::get("github:pat")`, rewrite `report::generate`).
      Demonstrates the "domain CLI on starter-cli" pattern as a
      counterpart to `examples/minimal`'s "domain server + CLI" shape.

Exit criteria for the wiring half — met today: `cargo test --workspace`
green; `cargo clippy --workspace --all-features --tests -- -D warnings`
clean; a binary using `ServerBuilder` actually serves `/health`,
`/metrics`, and `/openapi.json` against a real listener; MCP `tools/list`
+ `tools/call` over a registered `ToolRegistry` returns the registered
tools and invokes them; `starter-cli health --base-url ...` round-trips
against that same server.

---

## Phase 6 — TypeScript: codegen and the missing brain

- [x] Picked `openapi-typescript` (SCOPE open question 4). Type-only
      output; `starter-ui-core` owns the hooks. Wired as the `codegen`
      script in
      [packages/starter-client-ts/package.json](packages/starter-client-ts/package.json).
- [x] Source of truth for codegen: a checked-in workspace-root
      [openapi.json](openapi.json) snapshot. Generated by a Rust
      snapshot test in
      [crates/starter-auth-users/tests/openapi_snapshot.rs](crates/starter-auth-users/tests/openapi_snapshot.rs) —
      run with `UPDATE_SNAPSHOTS=1` to refresh. The canonical document
      is built from `utoipa::path` derives on the three `/auth/*`
      handlers (login / logout / me) plus the spi DTOs (`Role`,
      `Problem`) as components. New module
      [crates/starter-auth-users/src/openapi.rs](crates/starter-auth-users/src/openapi.rs)
      exposes `openapi()` so consumers can serve or merge it directly.
- [x] `src/generated/index.ts` now real codegen output (paths +
      components keyed on operation_id). Endpoints in
      [packages/starter-client-ts/src/endpoints/auth.ts](packages/starter-client-ts/src/endpoints/auth.ts)
      re-export the generated types instead of hand-rolling them, and
      `logout()` echoes the `starter_csrf` cookie back as
      `X-CSRF-Token`. `StarterError` gained a `fromResponse(res)` ctor
      that parses RFC 7807 problem bodies. CI drift gate is now live
      in [.github/workflows/ci.yml](.github/workflows/ci.yml) —
      runs the Rust snapshot test, regenerates the TS client, and
      fails on any diff.
- [x] Built `packages/starter-ui-core/` from empty. Surface:
  - [x] [package.json](packages/starter-ui-core/package.json) declares
        deps on `@nube/starter-client-ts`, `@tanstack/react-query`,
        `zustand`; React as a peer.
  - [x] `<AuthProvider>` + `useAuth()` in
        [src/auth/](packages/starter-ui-core/src/auth/) with three
        pluggable strategies (`sessionStrategy`, `tokenStrategy`,
        `externalStrategy`). Hook surface — `status`, `user`, `login`,
        `logout`, `refresh` — is identical across modes so app code
        doesn't branch on the auth flavour.
  - [x] Query-key helper in
        [src/query/index.ts](packages/starter-ui-core/src/query/index.ts):
        `starterQueryKey('auth', 'me')` → `['starter', 'auth', 'me']`,
        plus an `isStarterQueryKey` guard for namespaced invalidation.
        Lint-rule enforcement deferred to when ui-core grows its first
        useQuery call site — until then, the helper is the convention.
  - [x] `testing/` exports landed dependency-free in
        [packages/starter-ui-core/src/testing/](packages/starter-ui-core/src/testing/),
        exposed via the `@nube/starter-ui-core/testing` subpath.
        `createMockServer()` is a fetch shim plugged into
        `StarterClient` via the `fetch` option — mocks `/auth/me`,
        `/auth/login`, `/auth/logout` with mutable state. msw was
        skipped: its devDep tree is large and the three-route surface
        we mock here doesn't justify it. `createAuthWrapper()` returns
        a `({ children }) => ReactNode` for RTL-style
        `render(ui, { wrapper })`; RTL itself stays a consumer choice
        (devDep, not a peer).
- [x] `packages/starter-ui-kit/` left as the full shadcn dump (consumer-
      level decision: trimming risks breaking apps that already pull
      from the kit). Added [README.md](packages/starter-ui-kit/README.md)
      documenting the HugeIcons (`@hugeicons/react` +
      `@hugeicons/core-free-icons`) choice and the rationale.

Exit criteria — met: a consumer can `pnpm add @nube/starter-client-ts
@nube/starter-ui-kit @nube/starter-ui-core`, mount `<AuthProvider
client strategy={sessionStrategy}>`, call `useAuth().login({ kind:
'credentials', email, password })`, and read `useAuth().user` — all
without touching this repo. `cargo test --workspace`, `cargo clippy
--workspace --all-features --tests -- -D warnings`, `pnpm -r typecheck`,
and `pnpm -r build` all pass.

---

## Phase 7 — Docker, docs, and the final polish

- [x] `docker/Dockerfile.template` parameterised via `BINARY_NAME` and
      `FEATURES` build args (SCOPE 261–263). Two-stage build:
      `rust:1.83-slim-bookworm` for compilation,
      `gcr.io/distroless/cc-debian12` for runtime (keeps libc + libssl
      for reqwest / sqlx without shipping a shell or package manager).
      Default `ENTRYPOINT` is the binary; default `CMD` is `["serve"]`
      so a stock `docker run` boots the embedded server.
- [x] `docker/docker-compose.example.yml` — postgres + app reference
      with a `pg_isready` healthcheck so `app` doesn't race the DB
      boot. Standard `DATABASE_URL` / `STARTER_BIND_ADDR` / `RUST_LOG`
      environment plumbing. Documented as a starting point; consumers
      copy and edit `BINARY_NAME` + `FEATURES`.
- [x] CI: workspace check + clippy `-D warnings` + tests + pnpm build
      on every PR. Workflow at
      [.github/workflows/ci.yml](.github/workflows/ci.yml). Two jobs:
      `rust` (fmt, check --all-features, clippy --all-features --tests
      -D warnings, then test --workspace + per-crate feature-gated test
      runs for `starter-auth-token --features sqlite`,
      `starter-auth-users --features sqlite`, `starter-server
      --features testing`) and `pnpm` (typecheck + build with
      `actions/setup-node@v4` + `pnpm/action-setup@v4`,
      cache enabled). Concurrency group cancels superseded runs on the
      same ref. Third job `openapi-drift` runs the snapshot test,
      regenerates the TS client, and fails on any diff against the
      checked-in `packages/starter-client-ts/src/generated/index.ts`.
- [x] README in each crate / package explaining its one job, deps,
      features, and a usage snippet. Landed: all 14 Rust crates
      (`crates/starter-*`), all three TS packages (`packages/starter-*`),
      plus [examples/minimal/README.md](examples/minimal/README.md).
      Workspace [README.md](README.md) at-a-glance table updated to
      list the auth / secrets / ai crates that were missing.
- [ ] Cut a `0.1.0` tag, publish to crates.io / npm. Lockstep major
      bumps per SCOPE 144–145. Ready when the consumer-facing API is
      frozen — currently everything compiles + tests pass + READMEs
      ship + CI is green with the `openapi-drift` gate live.

---

## Cross-cutting items (do as you go, not in a single phase)

- [x] Tests exist across the workspace. ~60+ tests pass across
      `starter-spi`, `starter-store-sqlite`, `starter-auth-token`,
      `starter-auth-users`, `starter-server`, `starter-cli`,
      `starter-mcp`, `starter-secrets-file`, `starter-observability`,
      and the `examples/minimal` end-to-end smoke. Backend-feature
      coverage runs in CI under explicit `-p crate --features …`
      invocations. TS side: vitest covers `starter-client-ts` (8
      tests: error parsing + endpoint wire contracts) and
      `starter-ui-core` (16 tests: query-key namespacing, mock
      server, end-to-end AuthProvider flow under jsdom + RTL).
      `pnpm -r test` runs in CI alongside typecheck and build.
- [x] No `todo!()` remains on any production path. The auth bodies and
      observability/server middleware all carry real implementations.
      The two CLI prompts (`prompt::password`, `prompt::confirm`) that
      previously held `todo!()` placeholders now have real impls —
      `rpassword::prompt_password` for the password path; a plain
      stdin read with `[y/N]` default-no for confirm. `rpassword` is a
      workspace dep.
- [x] Supply-chain visibility via `cargo-audit` wired into CI as the
      `audit` job. `.cargo/audit.toml` documents the two ignores
      (`RUSTSEC-2023-0071` rsa via unused `sqlx-mysql`,
      `RUSTSEC-2025-0111` tokio-tar via dev-deps-only testcontainers)
      with paper-trail rationale. Bumped `prometheus` 0.13 → 0.14 to
      retire the `protobuf 2.x` transitive (`RUSTSEC-2024-0437`).
- [x] Every public `pub enum *Error` in the workspace is now
      `#[non_exhaustive]` (15 enums across spi / config / client-rs /
      cli / auth-token / auth-users / secrets-file). Adding a new
      error variant no longer forces a major bump, which matters
      heavily for a 0.x scaffold that should iterate fast. Cost:
      cross-crate `match` sites against `starter_spi::Error` now need
      a fallback arm; the two in-tree sites (`starter-server`'s
      `into_response` and `status_for`) got `_ => internal / 500`
      catch-alls so future spi variants degrade gracefully until a
      dedicated mapping is added.
- [x] Audited `.unwrap()` / `.expect()` usage across all `crates/*/src`.
      Every remaining occurrence is one of: (a) inside `#[cfg(test)]`
      or a `testing` module, (b) standard `Mutex.lock().expect(...)`
      panic-on-poisoned (idiomatic), (c) infallible-by-construction
      with a documenting message — `serde_json::to_vec(&(&str,&str))`,
      `Stdio::piped` -> `child.stdout.take()`, or upstream-lifted
      openai-spec message builders. No production-path fixes needed;
      revisit if a new call site adds a fallible path.
- [ ] Keep this file in sync — when a checkbox flips, edit it in the
      same commit that lands the work.

---

## Phase 8 — Tools and services (third-party integrations)

Scope: [DOCS/tools/scope/SCOPE.md](DOCS/tools/scope/SCOPE.md). Add a
family of sibling crates that wrap third-party providers (Gmail,
Slack, Telegram, …) as `Tool`s (one-shot request/response) or new
`Service`s (long-running listeners). One crate per integration, no
mega-crate, no cargo-feature matrix. Selected by Cargo dependency,
constructed in the consumer's `main.rs`.

Order: SPI types land first (everything else depends on them), then
the registry + supervision shell, then the first three provider
crates, then the surface adapters that expose tools over REST / CLI.

### Phase 8a — `starter-spi` additions

- [ ] Add `service::Service` trait (`async fn start(&self, ctx:
      ServiceContext) -> SpiResult<ServiceHandle>` + `name()`).
      Object-safe; covered by `tests/compile.rs`.
- [ ] Add `service::EventSink` trait (`async fn emit(&self, kind:
      &str, payload: serde_json::Value) -> SpiResult<()>`). Object-
      safe. Ship a blanket impl for `tokio::sync::broadcast::Sender<T>
      where T: From<(String, Value)>` and a `FanOut(Vec<Arc<dyn
      EventSink>>)` helper.
- [ ] Add `service::ServiceContext` (`#[non_exhaustive]`) with `metrics:
      Arc<prometheus::Registry>`, `shutdown: tokio::sync::watch::
      Receiver<bool>`, `sink: Arc<dyn EventSink>`.
- [ ] Add `service::ServiceHandle { join: JoinHandle<SpiResult<()>> }`.
      No shutdown sender — the registry owns the single
      `watch::Sender<bool>` and fans receivers out via
      `ServiceContext.shutdown` (R2/R9 in the scope doc).
- [ ] Re-export `secrecy::SecretString` from `starter-spi` so provider
      crates depend on the re-export, not `secrecy` directly (R5).
      Add `secrecy` to `[workspace.dependencies]`.
- [ ] Bump `starter-spi/tests/compile.rs` to lock the new exports:
      `Service` + `EventSink` object-safe, `ServiceContext` has the
      v1 fields, `SecretString` reachable.

### Phase 8b — `starter-services` registry crate

Sibling to `starter-cli` / `starter-mcp`. Holds the `ServiceRegistry`
and the supervision/shutdown plumbing — purely infrastructural, no
provider knowledge.

- [ ] Create `crates/starter-services/`. Single dep on `starter-spi`
      plus `tokio`, `tracing`, `prometheus`.
- [ ] Implement `ServiceRegistry::new()`, `.register(impl Service)`,
      `.start_all(metrics, sink) -> RunningServices`. `start_all`
      owns the single `watch::Sender<bool>`, hands cloned receivers
      to each `Service::start` via `ServiceContext`, returns a
      `RunningServices` guard.
- [ ] `RunningServices::shutdown(deadline)` flips the watch and awaits
      every `JoinHandle` with the configured deadline (default 5 s,
      override via builder). Returns a per-service exit summary.
- [ ] Restart policy: registry **never auto-restarts** (R9). On
      `JoinHandle` resolving to `Err`, record on the service span,
      increment `starter_service_restarts_total{service}`, and mark
      the service stopped. Document the `RestartingService<S: Service>`
      adapter pattern in the README for consumers who want supervised
      restart; do not ship the adapter in v1.
- [ ] Standard service metrics registered on the supplied
      `prometheus::Registry`: `starter_service_events_total{service,
      kind}`, `starter_service_restarts_total{service}`,
      `starter_service_running{service}` (gauge).
- [ ] Integration test in `tests/lifecycle.rs`: a `NoopService` that
      sleeps until `ctx.shutdown.changed()`, registered, started,
      shut down within 200 ms; gauge flips 1 → 0; second test where
      a service returns `Err` and the registry records the restart
      counter without auto-restarting.

### Phase 8c — Observability helpers

These live in `starter-observability` so every provider crate has
exactly two starter deps (`starter-spi` + `starter-observability`,
per R7).

- [ ] Add `tool_metrics(registry, tool_name)` returning a struct
      with `latency: Histogram`, `errors: Counter`. Provider crates
      register once at `Tool::new` time.
- [ ] Add `service_metrics(registry, service_name)` returning
      `events: CounterVec<&[&str]>`, `restarts: Counter`, `running:
      Gauge`. The registry crate (Phase 8b) calls this; provider
      crates use the returned handles via `ServiceContext.metrics`.
- [ ] Tracing span helpers: `tool_span(name)` / `service_span(name)`
      with stable field names (`tool.name` / `service.name`), used
      by both provider crates and the registry.

### Phase 8d — First three providers

Each crate ships: `Config` struct with `SecretString` fields, at
least one `Tool` impl (and where applicable a `Service` impl),
a README mirroring the notes example's "how it's extended" table,
and one integration test against a `wiremock`-style mock HTTP
server (no live network in tests).

Mine
`/home/user/code/rust/codeless-workspace/codeless/crates/codeless-{slack,telegram,bot-core,tools}`
for working code (local reference, NubeDev only) — lift
implementations, not architecture.

- [ ] `crates/starter-tool-gmail/` — `GmailSendTool` (send via Gmail
      API with refresh-token auth). Service deferred until a real
      consumer asks for Gmail watch.
- [ ] `crates/starter-tool-slack/` — `SlackPostTool` (chat.postMessage)
      + optional `SlackUpdateTool` (chat.update).
- [ ] `crates/starter-service-slack/` — `SlackSocketModeService`.
      Publishes `slack.message`, `slack.app_mention`, etc. via the
      `EventSink` from `ServiceContext`.
- [ ] `crates/starter-tool-telegram/` — `TelegramSendTool`
      (sendMessage).
- [ ] `crates/starter-service-telegram/` — `TelegramBotService`
      (long-poll `getUpdates`). Webhook variant deferred.
- [ ] Add all five crates to workspace members + `[workspace.
      dependencies]`.

### Phase 8e — Surface adapters

`Tool`s already reach MCP for free via `starter-mcp`. The other
surfaces need thin generic adapters so a registered tool is
callable everywhere without per-tool wiring.

- [ ] `starter-server`: add `tools_router(tools: Arc<ToolRegistry>)
      -> Router<S>` exposing `POST /tools/{name}` (one generic
      endpoint dispatches every registered tool). Auth via the
      consumer's `with_principal` wrap, same as `/mcp`.
- [ ] `starter-cli`: add a generic `tools call <name> --input <json>`
      subcommand registered by `register_starter_defaults` (alongside
      `health` / `openapi`). Pretty-prints the tool's JSON result.
- [ ] SSE bridge in `starter-server`: `events_sse_router(rx:
      broadcast::Receiver<ServiceEvent>) -> Router<S>` exposing
      `GET /events`. Consumer-built `EventSink` fan-outs into this
      receiver. Backpressure: lagged subscribers receive a `Lagged`
      event and reconnect; consistent with the notes example's
      `/notes/stream`.
- [ ] (Deferred to consumer for now) gRPC unary `CallTool` RPC —
      pattern documented in the notes example; no starter-side crate.

### Phase 8f — Example consumer + smoke tests

- [ ] Extend `examples/notes/` (or a new `examples/relay/`) with one
      tool registration + one service registration, end-to-end. The
      e2e test exercises: register tool → call via `/tools/{name}`,
      register service → emit event → observe via `/events` SSE.
- [ ] CI gate `starter-spi-deps-drift`: runs `cargo tree -p starter-spi
      --edges normal`, diffs against `DOCS/tools/scope/starter-spi-deps.baseline.txt`,
      fails on any change (the baseline updates only when `starter-spi`
      itself changes, in the same commit). Path-stripping rule
      documented in the workflow.
- [ ] Run the 5-point smoke test from SCOPE.md (no dep leakage, no
      special-case wiring, config-guarded construction, swappable
      secrets backend, bounded shutdown) against each provider crate
      before merging it.

Exit criteria: a binary depending on `starter-spi`, `starter-server`,
`starter-services`, `starter-tool-slack`, and `starter-service-slack`
can register both, run them, post a Slack message via `POST
/tools/slack_post`, receive a Slack event over `GET /events`, and
shut down cleanly inside the 5-second deadline. Adding
`starter-tool-telegram` is a one-line `Cargo.toml` change plus an
`if cfg.telegram.enabled { … }` block in `main.rs` — no other code
touched.
