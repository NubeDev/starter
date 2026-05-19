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
- [ ] Fix utoipa-5 incompatibilities in `starter-spi`:
  - `crates/starter-spi/src/paging/page.rs:10` — remove
    `#[aliases(PageOfString = Page<String>)]`. The attribute does not
    exist in utoipa 5; if a named alias is needed, use the v5
    `#[schema(as = ...)]` form or define a concrete newtype.
  - `crates/starter-spi/src/paging/cursor.rs` — derive `ToSchema` (and
    by extension `PartialSchema`) on `Cursor` so `Page<T>` compiles.
- [ ] Wire missing `@hugeicons/*` packages, or replace those icons.
      `packages/starter-ui-kit/src/components/ui/{breadcrumb,checkbox,
      command,context-menu,dialog,dropdown-menu,menubar,select,sheet,
      spinner}.tsx` import `@hugeicons/react` and `@hugeicons/core-free-
      icons` but neither is declared in `package.json`. Either add both
      to `dependencies`, or swap to the icon set the rest of the kit
      uses (lucide-react is the shadcn default).
- [ ] Add a `build` script to `packages/starter-ui-kit/package.json`.
      Today it only has `typecheck`; `pnpm -r build` skips it silently.

Exit criteria: `cargo check --workspace`, `cargo clippy --workspace --
-D warnings`, `pnpm -r build`, `pnpm -r typecheck` all pass.

---

## Phase 1 — Make `starter-spi` shape-complete

`starter-spi` is the contracts crate; everything downstream is shaped by
it. Closing the gaps here is what unblocks Phases 2-4.

- [ ] Move `Role` into `starter-spi`. Currently in
      `crates/starter-auth/src/role/kind.rs:8`. `Principal` carries a
      role per SCOPE 339–340, so the type belongs with `Principal`.
- [ ] Move `Scope` into `starter-spi`. Currently in
      `crates/starter-auth/src/scope/kind.rs:13`.
- [ ] Add a `role` field to `Principal`
      (`crates/starter-spi/src/auth/principal.rs:10`). Today it has
      `subject`, `scopes`, `extra` — no role. SCOPE: `Principal {
      user_id, role, scopes }`.
- [ ] Add `trait SecretStore` to `starter-spi` (SCOPE 342–346). Sync,
      `get(name) -> Option<Secret>`, `put(name, value)`, `delete(name)`,
      plus a `ready() -> bool` probe (SCOPE 562–563 implies it for the
      keyring impl). Decide the `Secret` type (likely a small newtype
      around `String` or `SecretString`) and document zeroize policy.
- [ ] Add `trait AiRunner` to `starter-spi` (SCOPE 349–353). Bring across
      the associated types unchanged from `codeless-workspace/ai-
      runner` so the lift in Phase 4 is a copy: `Provider`,
      `RunnerInput::{Cli(CliCfg), Rest(RestCfg)}`, `Event`, `RunResult`,
      `RunnerError`. Trait surface: `provider() -> &Provider`,
      `ready() -> bool`, `run(input, session_id, on_event, cancel) ->
      RunResult`.
- [ ] Decide `Authenticator` signature (SCOPE open question 3). Today
      it takes `&str` credential
      (`crates/starter-spi/src/auth/authenticator.rs:16`). Pick between
      `&http::request::Parts` (cheap, transport-coupled) and a richer
      `AuthContext` that pre-parses cookie + bearer. Document the
      choice in the trait's doc-comment.
- [ ] Decide `SecretStore` sync vs async (SCOPE open question 5). Default
      to sync; document the `block_in_place` cost path for future
      network-backed impls.
- [ ] Round out paging primitives. SCOPE 332 lists `Sort` and `Filter`
      as top-level primitives; today only `Direction` and `Predicate`
      exist (`crates/starter-spi/src/sort/`, `.../filter/`). Either add
      `Sort { field, direction }` and `Filter { predicates: Vec<...> }`
      or update SCOPE to match what's actually needed.
- [ ] Add `tests/compile.rs` real assertions. Today the file is a stub.
      Use it to lock the trait shapes (object-safety where required,
      bound checks, `impl ToSchema` round-trips).

Exit criteria: `starter-spi` exports `Role`, `Scope`, `SecretStore`,
`AiRunner` (+ associated types), `Principal { user_id, role, scopes }`;
no `starter-*` Cargo dep; compile test covers every public trait.

---

## Phase 2 — Storage actually stores

The store crates have the right *shape* but the migration runner is a
no-op and there's no actual SQL anywhere in the repo. Until this lands,
no auth crate can persist anything.

- [ ] Implement the namespaced migration runner in
      `crates/starter-store-sqlite/src/migrate/runner.rs:19` and the
      sibling at `crates/starter-store-postgres/src/migrate/runner.rs`.
      SCOPE 86–90: each source writes to its own
      `_sqlx_migrations_<source>` table so consumer migrations don't
      collide with starter ones. Settle the API per SCOPE open
      question 2 (builder vs macro vs `with_source` chain).
- [ ] Add `crates/starter-store-postgres/src/pool/connect.rs` mirroring
      sqlite's. Today there's a pool wrapper but no `connect(url)`.
- [ ] Real cursor encoding. `paging/cursor_codec.rs` today builds
      `format!("{sort}|{id}")`; replace with a stable base64url
      round-trip with a version byte so the format can evolve.
- [ ] Settle `Repository<T>` derive scope (SCOPE open question 1).
      Recommended scope: CRUD + paging + optimistic-locking version
      bumps, nothing more. Either land the derive macro or document
      it as "deferred until first consumer".
- [ ] Implement `testing::with_database` in
      `crates/starter-store-postgres/src/testing/with_database.rs:14`
      using `testcontainers`. Feature-gated `testing` already exists.
- [ ] Write the first migrations:
  - `crates/starter-store-sqlite/migrations/starter/` and
    `.../starter-postgres/migrations/starter/` — placeholder source so
    the runner has something real to run on first invocation.
- [ ] Integration tests in each store crate that actually run a
      migration end-to-end against the in-memory / containerised DB.

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

- [ ] Split the existing `starter-auth` crate:
  - `crates/starter-auth-token/` — pending+claimed records, claim flow,
    bearer verification. Lift from `NubeDev/token-service` as SCOPE
    439–440 directs.
  - `crates/starter-auth-users/` — argon2 passwords, cookie sessions,
    API tokens, role/scope guards, `/auth/login|me|logout` handlers.
    Receives most of what's in today's `starter-auth/`.
  - Delete the old `starter-auth` crate once the move is complete; remove
    it from `[workspace.dependencies]` in the root `Cargo.toml`.
- [ ] Update workspace `Cargo.toml` members and `[workspace.dependencies]`
      to list the two new crates.
- [ ] Implement `starter-auth-token` end-to-end:
  - [ ] First-boot claim flow: `claim_token` (32B base64url) generated
        on first start, stored in `ClaimStore` and in `SecretStore` at
        key `auth-token:pending` when one is wired (SCOPE 487–492).
  - [ ] `POST /auth/claim` handler: consumes pending token, stores
        SHA-256 digest of issued `owner_token`, returns the raw token
        once. Returns `AlreadyClaimed` on second attempt (SCOPE
        444–456).
  - [ ] `Authenticator` impl: constant-time compare of presented bearer
        against stored digest, returns `Principal { user_id: claim_id,
        role: Admin, scopes: vec![] }`.
  - [ ] Two migrations under source `starter_auth_token`:
        `starter_auth_token_pending`, `starter_auth_token_claimed`
        (single-row each per SCOPE 478–482).
  - [ ] `regenerate_claim_pending(store)` reset path + epoch bump
        (SCOPE 473–476). Exposed as `starter-cli reset --force`.
  - [ ] Integration tests covering claim, replay-reject, factory-reset,
        epoch invalidation.
- [ ] Implement `starter-auth-users` end-to-end. Today every body is
      `todo!()` (8 calls across `token/`, `password/`, `session/`,
      `admin/`). Required pieces:
  - [ ] argon2id password hash + verify (`password-auth` crate)
        replacing `password/{hash,verify}.rs` stubs.
  - [ ] Session lifecycle on `tower-sessions` + `axum-login`, DB-backed
        store so logout actually invalidates. `session/{issue,
        revoke,cookie}.rs` bodies.
  - [ ] API token issue + verify + revoke against
        `starter_auth_users_tokens` (id, user_id, hashed_token, scopes,
        last_used_at, expires_at, revoked_at). Bodies in `token/*.rs`.
  - [ ] Wire `/auth/login`, `/auth/me`, `/auth/logout` handlers and
        un-comment them in `routes/router.rs` (today returns
        `Router::new()`).
  - [ ] `Authenticator` impl that dispatches between cookie session and
        bearer token, both resolving to the same `Principal`.
  - [ ] Three migrations under source `starter_auth_users`:
        `starter_auth_users_users`, `_sessions`, `_tokens`.
  - [ ] Add CSRF protection on cookie-authenticated mutating routes.
        Not present today; design now while there are no handlers.
  - [ ] `starter-cli admin create --email --role admin` bootstrap path
        (today `admin/create_admin.rs:11` is `todo!()`).
  - [ ] Integration tests for both credential paths.
- [ ] Move `require_role` / `require_scope` middleware factories to
      where SCOPE puts them. SCOPE 460 says they live in
      `starter-server`, parameterised over the `Authenticator` trait;
      today they're in `starter-auth/guard/`. Either move them to
      `starter-server` or update SCOPE to keep them with auth.

Exit criteria: a binary can pick `starter-auth-token` and run the
claim → owner-token → bearer-auth flow with real persistence; a
different binary can pick `starter-auth-users` and run login / me /
logout + API tokens. The two are mutually exclusive via cargo features
in the consumer's `Cargo.toml`.

---

## Phase 4 — Secrets and AI

These two areas were redesigned after the original scaffold. Nothing of
them exists in the repo today.

- [ ] Create `crates/starter-secrets-keyring/` (SCOPE 189–193, 556–563).
      Wraps the `keyring` crate. Service name = binary crate name;
      key namespace `<binary>:<starter-component>:<key>`. `ready()`
      returns `false` in CI / headless containers.
- [ ] Create `crates/starter-secrets-file/` (SCOPE 195–201, 565–572).
      age-encrypted single file under `$XDG_DATA_HOME/<binary>/
      secrets.age`. Identity from `STARTER_SECRETS_KEY`, config path,
      or first-run generation with a printed warning.
- [ ] Add both to workspace members + `[workspace.dependencies]`.
- [ ] Wire `starter-auth-token` to read/write `auth-token:pending`
      through `SecretStore` when one is supplied (SCOPE 488–492).
- [ ] Create `crates/starter-ai/` by lifting `codeless-workspace/ai-
      runner` (SCOPE 203–210, 582–631; open question 7 picks "clean
      lift" so this becomes the source of truth). Each provider behind
      its own feature, all default-off:
  - [ ] `provider-claude` via `claude-wrapper` (pin `=0.5.1` per SCOPE
        820–824).
  - [ ] `provider-codex`, `provider-copilot` CLI wrappers.
  - [ ] `provider-anthropic` via `anthropic-ai-sdk`.
  - [ ] `provider-openai` via `async-openai`.
  - [ ] `Registry::with_defaults()` populating every enabled provider.
  - [ ] Secret integration: `SecretStore::get("ai:<provider>:api_key")`
        with env-var fallback (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`).
  - [ ] Cancellation via `CancellationToken`: `kill_on_drop(true)` for
        CLI subprocesses, body tear-down for REST.
  - [ ] Decide canary-CI repo location for the `claude-wrapper`
        stream-json drift check (SCOPE open question 6; recommended
        separate repo).
- [ ] Add `starter-ai` to workspace + `[workspace.dependencies]`.

Exit criteria: the "headless appliance" smoke test (SCOPE 735–741)
passes — a binary built with `starter-auth-token` + `starter-secrets-
file` + `starter-ai` (provider-anthropic only) does **not** pull
`starter-auth-users`, `starter-secrets-keyring`, `provider-claude`,
OpenAI/Codex/Copilot deps.

---

## Phase 5 — Server, CLI, MCP: finish the wiring

- [ ] Wire CORS + tracing middleware in
      `crates/starter-server/src/builder/server_builder.rs:51` (today a
      commented placeholder).
- [ ] `init_tracing` should return a guard
      (`crates/starter-observability/src/tracing/init.rs`); today it
      returns `Result<()>` and the file-appender drop is lost on early
      return paths.
- [ ] Implement `StandardMetrics::register`
      (`crates/starter-observability/src/metrics/standard.rs:22`),
      `request_id_layer` (`.../middleware/request_id.rs:26`) and
      `latency_layer` (`.../middleware/latency.rs:8`).
- [ ] Implement `starter-mcp` dispatch
      (`crates/starter-mcp/src/server/dispatch.rs:30` — today always
      `method_not_found`). Route `tools/list` and `tools/call` to the
      registry; call `Authenticator` from spi.
- [ ] Flesh out `starter-cli` commands. Today `health` and `openapi`
      bodies are empty (`crates/starter-cli/src/commands/health.rs:21`,
      `.../openapi.rs:24`). Implement them by calling
      `starter-client-rs`, not raw reqwest. Add `serve` and `migrate`
      built-in commands SCOPE 240 promises.
- [ ] Add `[[example]]` or `examples/` directories:
  - [ ] `examples/minimal/` — server + sqlite + cli, one resource.
  - [ ] `examples/full/` — server + postgres + mcp + react admin +
        docker.
  - [ ] `examples/gh-report/` — skeleton for the GitHub reporting tool.
  - Re-add the three to `[workspace] members` once they exist.

Exit criteria: `starter-cli health --url ...` actually hits a running
server; `starter-cli openapi` dumps the schema; MCP `tools/list` over
stdio returns the registered tools; all three examples build and
`cargo run` from each example does what its README says.

---

## Phase 6 — TypeScript: codegen and the missing brain

- [ ] Pick TS codegen tool (SCOPE open question 4): `openapi-typescript`
      (light, type-only) vs `orval` (heavier, generates react-query
      hooks). Recommendation: `openapi-typescript` so `starter-client-
      ts` stays thin and `starter-ui-core` owns the hooks.
- [ ] Implement the `codegen` script in
      `packages/starter-client-ts/package.json`. Root `package.json`
      already references `pnpm --filter @nube/starter-client-ts run
      codegen` but the script doesn't exist in the child package.
- [ ] Generate `src/generated/index.ts` from the server's OpenAPI doc;
      today it's a hand-written stub with three types and a `TODO
      (codegen)`. Wire a CI check that fails on drift (SCOPE 141–142).
- [ ] Build `packages/starter-ui-core/` from empty. Today the dir has
      no `package.json` and no source. Required surface per SCOPE
      117–134, 672–680:
  - [ ] `package.json` declaring deps on `@nube/starter-client-ts`,
        `@tanstack/react-query`, `zustand`; React as a peer.
  - [ ] `<AuthProvider>` + `useAuth()` hook with pluggable strategies:
        `sessionStrategy` (cookie endpoints), `tokenStrategy` (bearer),
        `externalStrategy` (IdP). Identical hook surface across modes
        so app code doesn't branch.
  - [ ] Query-key namespacing helper: every starter-owned react-query
        key prefixed `['starter', ...]`. Lint rule in CI.
  - [ ] `testing/` exports `MockServer` (msw-backed) and
        `renderWithProviders`.
- [ ] Trim or reshape `packages/starter-ui-kit/`. The current dir is a
      full shadcn dump (38 components in `src/components/ui/`). Decide
      whether the kit ships the lot or only what SCOPE actually names
      (primitives, theme, visual hooks). At minimum: declare the
      `@hugeicons/*` deps so it compiles; pick a single icon set and
      document it.

Exit criteria: a consumer can `pnpm add @nube/starter-client-ts
@nube/starter-ui-kit @nube/starter-ui-core` and stand up a login form +
authenticated data fetch without touching this repo (the "block author"
smoke test, SCOPE 743–748).

---

## Phase 7 — Docker, docs, and the final polish

- [ ] `docker/Dockerfile.template` parameterised via `BINARY_NAME` and
      `FEATURES` build args (SCOPE 261–263).
- [ ] `docker/docker-compose.example.yml` — server + postgres reference.
- [ ] CI: workspace check + clippy `-D warnings` + tests + pnpm build +
      OpenAPI/TS drift check on every PR.
- [ ] README in each crate / package explaining its one job, deps,
      features, and a 10-line usage snippet.
- [ ] Cut a `0.1.0` tag, publish to crates.io / npm. Lockstep major
      bumps per SCOPE 144–145.

---

## Cross-cutting items (do as you go, not in a single phase)

- [ ] Add `#[cfg(test)]` blocks or `tests/` directories to every crate.
      Today **zero** non-stub tests exist in the workspace.
- [ ] Replace every remaining `todo!()` in production paths (see audit
      §4 for the current list of 8 in `starter-auth` + 4 elsewhere).
- [ ] Audit `.unwrap()` / `.expect()` usage as code lands; today the
      only two are in test harnesses and acceptable.
- [ ] Keep this file in sync — when a checkbox flips, edit it in the
      same commit that lands the work.
