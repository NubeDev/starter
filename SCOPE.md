# starter — Scope

## One-line summary

`starter` is a set of small, focused libraries — Rust crates + React/TS
packages — that a new product imports to get a working **cli + server +
storage + MCP + admin UI** without forking, copying, or rebuilding the
plumbing each time.

It is **not a framework** and **not a template to clone**. A consumer
does `cargo add` + `pnpm add` for the pieces they need, implements a
small number of trait/interface seams, and owns their own domain code.
The boundary between "starter" and "consumer" is the same shape as the
boundary between `tokio` and an app that uses `tokio`: starter is a
dependency, not a parent project.

## Why this exists

Across several Rust projects (codeless, a coming GitHub reporting tool,
internal dashboards, etc.) the same load-bearing plumbing keeps being
re-implemented:

- A clap-driven CLI that wraps an HTTP client to drive the server.
- An axum server with OpenAPI emission and structured error handling.
- SSE for streaming endpoints; optional gRPC where bidi/perf matters.
- SQLite-or-Postgres behind a typed pool + migration runner so the
  same app runs on a laptop and in production.
- An MCP server exposing the app's domain to Claude / other agents.
- A Dockerfile + compose recipe that doesn't have to be re-invented.
- A React + Tailwind + shadcn admin shell with a light/dark/system
  theme switch.

Each of those pieces is small on its own. The cost is in *consistency*:
when each new project re-rolls them, they drift, the AI-assisted edits
land in the wrong layers, and the same bugs get re-fixed. `starter`
extracts each piece into a crate or package whose **one job** is named
on the tin.

## Hard rules (load-bearing)

These rules are why downstream consumers can mix-and-match. Break one
and the modularity collapses.

### R1 — One responsibility per crate/package

**Per file (hard):** ≤ 400 lines. **Per module:** ≤ ~10 public items.
**Per function (preference, not hard):** aim for ≤ 50 lines; a derive-
heavy axum router or a utoipa-decorated handler module will legitimately
go higher. The hard ceiling is the file. **No `utils`, `helpers`,
`common`, `misc` crates or modules** — name the concept. If a crate's
job needs the word "and" to describe, split it.

### R2 — `starter-spi` is the contracts crate; everything depends on it, it depends on nothing

Wire types and trait seams live in `starter-spi`: errors, IDs, paging
primitives, the `Tool` trait (MCP), the `Authenticator` trait + `Principal`
+ `Role`, the `SecretStore` trait, the `AiRunner` trait + its associated
event/result types, and the OpenAPI-derivable request/response DTOs.
Zero internal deps, zero runtime logic, zero HTTP, zero SQL. Every other
crate consumes it.

**Notably absent from `spi`:** there is no `Store` trait. See R4 for
why.

### R3 — Transport never contains domain logic

REST handlers, gRPC handlers, CLI commands, and MCP tool handlers are
each thin: extract → call domain function → shape result → return. The
canonical smoke test: *if I swap REST for gRPC tomorrow, how much of
this file changes?* If more than route wiring and DTO shaping — the
logic is in the wrong layer.

### R4 — Storage is typed building blocks, not a universal trait

There is no `Store` trait in `spi`. A universal `Store` trait would
have to abstract over resources `spi` cannot know exist, and any
consumer with a non-trivial query would either vendor SQL into a
starter crate or fork. Both outcomes break the model.

Instead, `starter-store-sqlite` and `starter-store-postgres` ship
**typed building blocks** the consumer composes their own repositories
from:

- A configured `sqlx::Pool` exposed via a thin wrapper that carries
  observability + tracing context.
- A migration runner that namespaces starter-owned migrations and
  consumer-owned migrations into **separate `_sqlx_migrations` tables**
  (one per source) so version numbers can't collide.
- Query helpers for the patterns starter itself uses (paging, cursor
  encoding, optimistic-locking version bumps, JSON column round-trip).
- An optional `Repository<T>` derive for simple CRUD when the consumer
  doesn't need custom SQL.

The rule, restated: **starter's own SQL lives only in `starter-store-*`
crates.** Consumer SQL against the exposed pool is fine and expected.
The wall is between *starter code* and *raw SQL*, not between
*consumer code* and *raw SQL*.

### R5 — Default-features minimal; opt-in everything else

Every crate's `default-features = []`. A consumer who doesn't want
Postgres / gRPC / MCP / the reference OpenAI provider pays nothing for
them — no transitive deps pulled, no code compiled. The features list
is the menu; the consumer's `Cargo.toml` is their order.

### R6 — TS client has zero React; UI-kit has zero I/O; UI-core owns the brain

`@nube/starter-client-ts` is a plain TS HTTP client usable from
Node/Bun/Deno/browser, with **no React**.
`@nube/starter-ui-kit` ships shadcn primitives + Tailwind tokens + the
theme-switch hook, with **no API calls, no stores, no hooks that do
I/O**. `@nube/starter-ui-core` is the "portable brain" — every hook,
provider, and store that talks to the server.

Within `ui-core`:

- **Route definitions belong to the consumer app**, not `ui-core`.
  `ui-core` exports hooks and providers; the consumer wires them into
  their router.
- **Auth flow lives in `ui-core`** as an `AuthProvider` + `useAuth`
  hook with pluggable strategies:
  - `sessionStrategy` — talks to `starter-auth-users`' cookie endpoints
    (`/auth/login`, `/auth/me`, `/auth/logout`). No token handling in JS.
  - `tokenStrategy` — for `starter-auth-token` deployments. Owner token
    is supplied once (env var, paste-in dialog, or out-of-band) and sent
    as `Authorization: Bearer …`. No login form.
  - `externalStrategy` — for consumers on an external IdP.

  The hook surface (`useAuth().principal`, `.login()`, `.logout()`) is
  identical across strategies so app code doesn't branch on auth mode.
- **Query-key namespacing is enforced.** Every starter-owned react-
  query key is prefixed `['starter', ...]`. Consumers prefix their own
  keys with their app name. Documented; lint-checked in CI.

### R7 — One source of truth for wire types; TS is codegen'd from Rust

`starter-spi` Rust types decorated with `utoipa::ToSchema` are the
source of truth. The OpenAPI document emitted by `starter-server`
drives codegen of `@nube/starter-client-ts`'s Zod schemas and TS
types. **Hand-edited TS wire types are forbidden.** `pnpm codegen` is
the supported workflow; CI fails on drift.

Versioning is add-only within a major for both surfaces; breaking
changes bump the major on both the Rust crate and the npm package in
lockstep.

### R8 — Comments explain *why*, never *what*. No session-progress chatter

Doc-comments on every public item explaining purpose, defaults, and
edge cases. No `// STAGE-1 done`, no `// FIXED:`, no emoji banners.
TODOs carry a name or ticket: `// TODO(ap): …`.

## Repo layout

```
starter/                                   <- this repo
  Cargo.toml                               <- workspace
  pnpm-workspace.yaml

  crates/
    starter-spi/                           <- R2: contracts. Zero deps.
                                              Error, Id, paging, Tool trait,
                                              Authenticator trait,
                                              OpenAPI-derivable DTOs.

    starter-config/                        <- layered config (env > file > default)
                                              built on figment. Generic over the
                                              consumer's config struct.

    starter-observability/                 <- tracing-subscriber setup,
                                              prometheus exporter, request-id +
                                              latency middleware factories.

    starter-server/                        <- axum app builder.
                                              Accepts consumer-built Routers and
                                              merges them. OpenAPI via utoipa.
                                              SSE helpers. Testing harness.

    starter-store-sqlite/                  <- typed building blocks on sqlx+sqlite:
                                              pool wrapper, namespaced migration
                                              runner, paging helpers, optional
                                              Repository<T> derive.
    starter-store-postgres/                <- same shape on sqlx+postgres.

    starter-mcp/                           <- MCP stdio server scaffold.
                                              Consumer registers Tool impls;
                                              this crate handles the protocol.

    starter-secrets-keyring/               <- OPTIONAL, feature-gated.
                                              SecretStore impl on the OS keyring
                                              (macOS Keychain, Windows Credential
                                              Manager, Linux Secret Service).
                                              For desktop / developer machines.

    starter-secrets-file/                  <- OPTIONAL, feature-gated.
                                              SecretStore impl backed by an
                                              encrypted file (age-encrypted, key
                                              from env or kernel keyring). For
                                              headless servers / containers
                                              where no desktop keyring daemon
                                              is available.

    starter-ai/                            <- OPTIONAL, per-provider features.
                                              Unified AI runner trait + Registry.
                                              CLI providers (claude-wrapper,
                                              codex, copilot) and REST providers
                                              (anthropic-ai-sdk, async-openai).
                                              Streaming events, cancellation,
                                              backpressure. Lifted from
                                              codeless-workspace/ai-runner.

    starter-auth-token/                    <- OPTIONAL, feature-gated.
                                              Authenticator impl for headless /
                                              single-operator deployments. One
                                              owner-token, no users, no sessions.
                                              First-boot claim flow: pending
                                              one-time claim_token → owner_token
                                              (32B base64url, SHA-256 stored).

    starter-auth-users/                    <- OPTIONAL, feature-gated.
                                              Authenticator impl for multi-user
                                              apps: local users (argon2) + cookie
                                              sessions for browsers + hashed API
                                              tokens for machine clients. Three
                                              built-in roles (reader/writer/admin)
                                              and an extensible Scope set.

                                              starter-auth-token and
                                              starter-auth-users are mutually
                                              exclusive — a consumer picks one
                                              (or neither, or writes their own
                                              Authenticator).

    starter-client-rs/                     <- Rust HTTP client. Zero
                                              starter-server dep — shares
                                              starter-spi types only.

    starter-cli/                           <- clap building blocks + a default
                                              set of subcommands (serve, migrate,
                                              health, openapi). The consumer
                                              composes their own binary; this
                                              crate is a library, not a binary.

    starter-grpc/                          <- tonic gRPC server scaffold. Sibling
                                              of starter-mcp: consumer registers
                                              `Tool` impls, this crate surfaces them
                                              as `starter.tools.v1.Tools`
                                              (ListTools + CallTool). Optional
                                              Authenticator-gated bearer auth.
                                              Optional `reflection` feature.

  packages/
    starter-client-ts/                     <- generated from OpenAPI. Zod
                                              schemas. R6: zero React.

    starter-ui-kit/                        <- shadcn primitives + Tailwind preset
                                              + theme-switch hook + design tokens.
                                              R6: zero I/O, zero stores.

    starter-ui-core/                       <- React hooks/providers/stores that
                                              talk to the server through
                                              starter-client-ts. AuthProvider
                                              lives here. The portable brain.

  docker/
    Dockerfile.template                    <- parameterized via build args
                                              (BINARY_NAME, FEATURES). Consumers
                                              copy or extend; documented as a
                                              starting point, not magic.
    docker-compose.example.yml             <- server + postgres reference.

  examples/
    minimal/                               <- server + sqlite + cli, one resource.
    full/                                  <- server + postgres + mcp + react admin
                                              + docker, end-to-end.
    gh-report/                             <- skeleton for the GitHub reporting
                                              tool: imports starter, defines its
                                              domain, nothing more.
```

## Dependency arrow (Rust)

```
starter-spi
   ↑          (everything depends on spi; spi depends on nothing)
   │
   ├── starter-config
   ├── starter-observability
   ├── starter-store-sqlite        ──┐
   ├── starter-store-postgres      ──┤ (consumer picks one or both)
   ├── starter-server              ──┼─→ starter-spi only; gets a pool +
   ├── starter-mcp                 ──┤    consumer-built Routers via DI
   ├── starter-auth-token          ──┤    (both impl Authenticator from spi;
   ├── starter-auth-users          ──┤    starter-server depends only on the
   │                                  │    trait. The two auth crates are
   │                                  │    mutually exclusive in any one
   │                                  │    binary.)
   ├── starter-secrets-keyring     ──┤    (both impl SecretStore from spi;
   ├── starter-secrets-file        ──┤    not mutually exclusive — a binary
   │                                  │    can use keyring on dev, file in
   │                                  │    prod via cargo features.)
   ├── starter-ai                  ──┤    (impls AiRunner trait from spi.
   │                                  │    Depends on SecretStore trait in
   │                                  │    spi for API keys, NOT on any
   │                                  │    concrete secrets-* crate — the
   │                                  │    consumer wires the impl.)
   ├── starter-client-rs
   └── starter-cli                 → starter-client-rs (NEVER raw HTTP)
```

**Never** the other way: no crate consumes `starter-server` except
example binaries and the consumer's own binary. The server is a
dead-end consumer of the spi, not a reusable parent.

## Dependency arrow (TypeScript)

```
starter-client-ts        (zero deps on UI; codegen'd from OpenAPI)
        ↑
   ┌────┴────┐
   │         │
starter-     starter-ui-core
ui-kit              ↑
   ↑                │
   └─── consumer app (Studio / admin / gh-report frontend)
```

`ui-core` consumes `client-ts` for I/O and `ui-kit` for visuals. A
consumer app consumes all three. **`ui-kit` never imports `client-ts`**
and **`client-ts` never imports React** — those are the load-bearing
walls.

## What each crate / package owns

### `starter-spi` (Rust)

- `Error`, `Result<T>` — domain-shaped errors, never HTTP-shaped.
- `Id<T>`, `Page<T>`, `Cursor`, `Sort`, `Filter` — generic primitives.
- `trait Tool` — MCP-tool shape: name, schema, invoke.
- `trait Authenticator` — resolves a request's credentials (cookie
  session, bearer token, or none) into an `Option<Principal>`. Lives
  here (not in `starter-server`) so MCP, gRPC, and any future transport
  share one auth seam.
- `Principal { user_id, role, scopes }`, `Role { Reader, Writer, Admin }`,
  `Scope` — the post-auth identity carried through the request. Wire
  type, not storage type.
- `trait SecretStore` — `get(name)`, `put(name, value)`, `delete(name)`
  for named secrets (API keys, owner tokens, signing keys). Sync
  trait — backends are local, no I/O latency to amortise. Lives in
  `spi` so `starter-auth-token` and `starter-ai` depend on the trait,
  not a specific backend.
- `trait AiRunner` — the streaming AI provider seam (CLI subprocess or
  REST). `provider() -> &Provider`, `ready() -> bool`, `run(input,
  session_id, on_event, cancel) -> RunResult`. The associated
  `Provider`, `RunnerInput { Cli(CliCfg) | Rest(RestCfg) }`, `Event`,
  `RunResult`, and `RunnerError` types live here too — so a consumer
  can store an `Arc<dyn AiRunner>` in their `AppState` and not depend
  on `starter-ai` at all if they write a custom runner.
- DTOs decorated with `utoipa::ToSchema` so OpenAPI emission is
  automatic.

### `starter-config`

Layered config (defaults → file → env → CLI flags) using `figment`.
Exposes a `Config` trait the consumer parameterises with their own
struct. No knowledge of HTTP, DB, or anything domain-shaped.

### `starter-observability`

`init_tracing(level, fmt)` returning a guard. Prometheus `Registry`
factory and an `axum::middleware::from_fn` request-id + latency
middleware that other crates can mount. **No log macros wrapped** —
consumers use `tracing` directly.

### `starter-server`

`ServerBuilder` that takes:

- one or more `axum::Router<AppState>` instances the consumer has
  already built (starter merges them via `Router::merge`),
- the consumer's `AppState` (which carries the pool, auth, and
  anything else),
- a `Vec<utoipa::openapi::PathItem>` (or a single `utoipa::OpenApi`)
  the consumer assembles from their handler annotations,
- an optional MCP attachment,
- a config.

The seam is **handing a Router to starter**, not implementing a
`Route` trait — that's how axum wants to be composed. starter-server
adds: OpenAPI emission at `/openapi.json`, health/metrics routes, CORS
+ tracing middleware, graceful shutdown. SSE helper module
(`sse::keep_alive`, `sse::from_stream`).

**Twenty-line handler ceiling** for handlers starter itself ships;
consumer handlers are governed by R1's file ceiling.

### `starter-store-sqlite` / `starter-store-postgres`

Each exposes:

- `Pool` — thin wrapper around `sqlx::Pool` carrying tracing context.
- `migrate(pool, source)` — runs migrations from one `MigrationSource`
  into a source-specific `_sqlx_migrations_<source>` table.
  Starter's own migrations use source = `starter`; consumers register
  their own source (e.g. `app`) and migrations into the same pool
  without version-number collisions.
- Paging helpers (`Cursor` encode/decode, `LIMIT/OFFSET` builders).
- Optional `#[derive(Repository)]` for the common case where the
  consumer just wants typed CRUD over one table.

Consumers writing custom SQL against the pool is **expected and
supported**; R4 only restricts starter's own SQL.

### `starter-mcp`

stdio-based MCP server loop. The consumer registers `Tool`
implementations; this crate handles the protocol, auth (via the
`Authenticator` from `spi`), and lifecycle. Zero domain code. **stdio
only for v1** — SSE/HTTP MCP transports added when a consumer needs
them.

### Auth crates (both OPTIONAL, mutually exclusive)

starter ships two default `Authenticator` implementations covering the
two shapes consumers actually have. A binary picks **one** via cargo
features, or neither (no auth → routes are public unless the consumer
wires their own middleware), or writes a custom impl behind the
`Authenticator` trait. The trait seam in `spi` never moves.

| | `starter-auth-token` | `starter-auth-users` |
|---|---|---|
| Use case | Headless appliance / single-operator tool / edge device | Multi-user app with a browser UI |
| Identities | One owner | Many users |
| Browser story | None — bearer only | Cookie sessions |
| Roles | None — token = full admin | reader / writer / admin + scopes |
| Storage | Two records (pending + claimed) | Users, sessions, tokens tables |
| Bootstrap | First-boot claim flow | `starter-cli admin create` |

### `starter-auth-token` (OPTIONAL, feature-gated)

A pre-shared-bearer-token `Authenticator` for the "no users, just keep
strangers out" case. Adapted from
[NubeDev/token-service](https://github.com/NubeDev/token-service): the
crypto, claim state machine, and `ClaimStore` trait already match
starter's R2/R3 shape and are lifted in mostly unchanged.

**First-boot claim flow.** On first start the server is *unclaimed*
and writes a one-time `claim_token` (32 random bytes, base64url, 43
chars) to its `ClaimStore`. Whoever presents that token to
`POST /auth/claim` with an `owner` label becomes the owner and
receives the long-lived `owner_token` once — it is **never persisted
raw**; only its SHA-256 digest is stored. The pending token is
consumed, so a second claim fails with `AlreadyClaimed`. This is the
TOFU (trust on first use) model: whoever boots the box first owns it.

**Authentication.** Every subsequent request carries
`Authorization: Bearer <owner_token>`. The `Authenticator` impl hashes
the presented token (constant-time) and compares against the stored
digest. On match it returns `Principal { user_id: <claim_id>,
role: Admin, scopes: vec![] }`. There is no other role — the owner
token is a full-admin credential, by design.

**Route protection.** Same `require_role(Admin)` middleware
factory as `starter-auth-users` (lives in `starter-server`, parameter-
ised over the trait):

```rust
Router::new()
    .route("/health", get(health))                  // unauthenticated
    .route("/auth/claim", post(claim))              // unauthenticated, one-shot
    .nest("/api", app_routes().layer(require_role(Admin)))
```

Routes without `require_role` are public; routes with it require the
owner token. There are no "reader" or "writer" routes in this model
— that's the line between the two crates.

**Factory reset.** `regenerate_claim_pending(store)` reopens the claim
window unconditionally; surfaced as `starter-cli reset --force` and
gated by physical/host access only (never an HTTP endpoint). Bumps an
`epoch` counter so prior owner tokens become invalid.

**Persistence.** Two records, no schema explosion:

- `starter_auth_token_pending` (single row: pending claim_token, or empty)
- `starter_auth_token_claimed` (single row: owner label,
  owner_token_hash, claim_id, epoch, claimed_at)

Shipped as starter migrations under source = `starter_auth_token`.
Works on sqlite and postgres unchanged. Footprint is small enough that
a consumer running on a Raspberry Pi pays nothing surprising.

**Secret storage.** When a `SecretStore` impl is wired in, the
pending `claim_token` is stored there (`auth-token:pending`) instead
of in the DB — pending claim tokens are short-lived bearer secrets
and belong in the same place as API keys. The hashed owner record
stays in the DB (it's a digest, not a secret).

**What it deliberately doesn't do.** No login page, no users table, no
sessions, no password reset, no MFA, no per-route role granularity.
A consumer who outgrows this either switches to `starter-auth-users`
or writes a custom `Authenticator`.

### `starter-auth-users` (OPTIONAL, feature-gated)

The default `Authenticator` for multi-user apps. Off by default — a
consumer who doesn't add the crate pays zero. Consumers who later move
to OIDC swap this crate for their own `Authenticator` impl; the trait
seam in `spi` does not move.

**Two credential paths, one `Principal`:**

- **Browser → cookie sessions.** Built on `axum-login` +
  `tower-sessions` with a DB-backed session store (so logout actually
  invalidates). `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`.
  Passwords hashed with argon2id via `password-auth`.
- **Machine clients (CLI, MCP, scripts) → API tokens.** Long-lived
  opaque tokens stored argon2-hashed in a `starter_auth_users_tokens`
  table: `id, user_id, hashed_token, scopes, last_used_at,
  expires_at, revoked_at`. Sent as `Authorization: Bearer …`.
  Revocable; not a JWT.

Both paths resolve to the same `Principal` so route guards never care
which mechanism the caller used.

**Route protection.** A `require_role(Role)` and `require_scope(Scope)`
middleware factory the consumer mounts per route or per router:

```rust
Router::new()
    .route("/items", get(list))                                 // public read
    .route("/items", post(create).layer(require_role(Writer)))
    .route("/items/:id", delete(remove).layer(require_role(Admin)))
```

Handlers extract `Principal` via an axum extractor for per-row checks
(ownership, tenant scoping if the consumer adds it).

**Persistence.** Owns its own tables (`starter_auth_users_users`,
`starter_auth_users_sessions`, `starter_auth_users_tokens`) shipped as
starter migrations under source = `starter_auth_users` via the
namespaced migration runner from `starter-store-*`. Works with sqlite
and postgres unchanged.

**Bootstrap.** First-run admin is created via the CLI
(`starter-cli admin create --email … --role admin`), never via an
unauthenticated HTTP endpoint. No self-service signup by default;
consumers add `/signup` if they want it.

**Deferred to v2:** MFA/TOTP, password reset email flow, OAuth social
login. The trait shape leaves room; the crate doesn't ship them in v1.

### Secret storage (`starter-secrets-keyring` / `starter-secrets-file`)

Both impl `SecretStore` from `spi`. A consumer picks one (or one per
build profile via features) and the rest of the stack — `starter-auth-
token`'s owner_token, `starter-ai`'s provider API keys, the consumer's
own signing keys — pulls secrets through the trait without knowing
which backend is mounted.

**`starter-secrets-keyring`** wraps the [`keyring`](https://docs.rs/keyring)
crate. Targets desktop / developer workstations. Service name is the
binary's crate name; secrets are namespaced as
`<binary>:<starter-component>:<key>` (e.g.
`gh-report:auth-token:owner_token`) so two starter-based apps on the
same machine don't collide. **No use in CI or headless containers** —
the OS keyring daemons are not available there; the crate's `ready()`
returns `false` and the consumer is expected to feature-swap to file.

**`starter-secrets-file`** stores secrets in a single
[age](https://github.com/FiloSottile/age)-encrypted file
(`$XDG_DATA_HOME/<binary>/secrets.age`). The age identity comes from,
in order: `STARTER_SECRETS_KEY` env var, a file path in the consumer's
config, or generation-on-first-run with a clear printed warning to
back it up. Intended for server / container deployments where the
keyring path doesn't work. **No HSM, no cloud KMS** — a consumer who
needs those writes their own `SecretStore` impl.

**Why a trait and not "just use env vars".** Env vars work for one
secret on one machine. As soon as you have multiple (owner token + AI
keys + signing key) and care about rotation, deletion, or "show me
what's stored", a uniform interface beats `std::env::var` calls
scattered across the codebase. The trait stays small enough that
"just env vars" remains a 20-line impl for a consumer who genuinely
only needs that.

### `starter-ai` (OPTIONAL, per-provider features)

Unified AI provider runner. Adapted from
[`codeless-workspace/ai-runner`](../codeless-workspace/ai-runner) — the
shape (trait `Runner`, `Provider` enum, `RunnerInput::{Cli, Rest}`,
streaming `Event` channel, `CancellationToken` for kill semantics)
already matches starter's R2/R3 design and is lifted in essentially
unchanged. The crate names move to the `starter-` prefix; the trait
moves into `starter-spi` so consumers can implement custom runners
without depending on this crate.

**Providers** (each behind its own cargo feature, all default-off):

- `provider-claude` — Claude Code CLI via `claude-wrapper` (pinned
  version because the binary's stream-json output is not a stable
  API; CI canary tracks upstream).
- `provider-codex` — OpenAI Codex CLI.
- `provider-copilot` — GitHub Copilot CLI.
- `provider-anthropic` — Anthropic REST API via `anthropic-ai-sdk`.
- `provider-openai` — OpenAI REST API via `async-openai`.

A consumer who only wants Anthropic REST adds
`features = ["provider-anthropic"]` and pays nothing for the CLI
wrappers or the OpenAI SDK.

**Registry.** `Registry::with_defaults()` returns an
`Arc<Registry>` populated with every provider whose feature is on;
the consumer keeps it in `AppState`. Per-provider `ready()` probes
disk (CLI binary discovery) or env (API key presence) without making
network calls — `ready() == true` means "the runner can attempt a
call", not "the upstream is healthy".

**Secret integration.** `starter-ai` reads provider API keys via
`SecretStore::get("ai:anthropic:api_key")` etc. when a `SecretStore`
is wired in; it falls back to env vars (`ANTHROPIC_API_KEY`,
`OPENAI_API_KEY`) when no store is configured. This is the seam that
makes "rotate one key" a one-liner instead of a deploy.

**Streaming + cancellation.** REST runners drive the run from async
and `await` each `send` (natural backpressure). CLI runners
`try_send` from the wrapper's sync callback and drop on overflow with
a `tracing::warn!` — drops are best-effort by design. Cancellation
via `CancellationToken` kills CLI subprocesses on drop
(`kill_on_drop(true)`) and tears down REST HTTP bodies; callers
typically observe the result within a few hundred milliseconds.

**What it deliberately doesn't do.** No prompt templating, no
retries, no rate limiting, no provider-failover policy, no "smart"
routing. Those are consumer concerns built on top — this crate is
the unified seam, not the orchestration layer.

### `starter-client-rs`

Reqwest-based HTTP client whose methods mirror `starter-server`'s
routes. Used by consumer CLI binaries and any Rust consumer that wants
to call a starter-based server. **Zero `starter-server` dep** — they
share `starter-spi` types only.

### `starter-cli`

A **library of clap building blocks**, not a binary. Ships:

- Built-in `Command` impls for `serve`, `migrate`, `health`, `openapi`
  that work against any starter-based server via `starter-client-rs`.
- A `CommandRegistry` the consumer's binary uses to assemble its own
  CLI from starter's commands + its own.

**The consumer composes their CLI binary in their own crate.** They
depend on `starter-cli` for the building blocks and on their own
domain crates for app-specific commands. `starter-cli` never depends
on consumer code; the consumer's binary depends on both.

### `starter-client-ts` (TS)

Plain `fetch`-based HTTP client. **Generated from the server's OpenAPI
document** via `pnpm codegen`. Zod schemas validate at the wire
boundary. **No React.** Works in Node / Bun / Deno / browser.

### `starter-ui-kit` (TS)

- shadcn primitives wrapped with the project's design tokens.
- Tailwind preset (`tailwind.preset.ts`) the consumer extends.
- `<ThemeProvider>` + `useTheme()` hook with `light | dark | system`
  modes, persisted to `localStorage`, listening to
  `prefers-color-scheme`.
- Visual-only hooks (`useViewport`, `useFocusTrap`) allowed.
- **No** React Query, no zustand, no fetches.
- Ships a neutral default token set; consumers override via the
  Tailwind preset extension API.

### `starter-ui-core` (TS)

Every hook / provider / store that touches the server. Built on
`@tanstack/react-query` for server state + `zustand` for client state.
Imports `starter-client-ts` for I/O. Owns `<AuthProvider>` +
`useAuth()`. **No app-shell pages, no router config** — consumers own
those. Query keys are prefixed `['starter', ...]`; consumers MUST
prefix their own keys with their app name.

## Testing seams

Every starter crate that has a non-trivial surface ships a test
harness so consumers don't reinvent them.

- **`starter-server::testing`** — `TestApp::spawn(state, routers)`
  returns a bound `axum::Router` + a `reqwest::Client` pointed at a
  random local port. Used by integration tests in consumer crates.
- **`starter-store-sqlite::testing::ephemeral()`** — returns a `Pool`
  backed by `:memory:` with all starter migrations applied. Consumers
  register their own migrations on top.
- **`starter-store-postgres::testing::with_database()`** — testcontainers-
  based fixture (feature-gated, off by default).
- **`starter-mcp::testing`** — in-memory transport pair so tool
  invocations can be tested without spawning a subprocess.
- **`starter-client-rs`** has a `mock` feature exposing a builder for
  recorded responses, used in CLI tests.

On the TS side, `starter-ui-core/testing` exports a `MockServer` that
intercepts `starter-client-ts` calls via `msw`, plus a
`renderWithProviders` helper that wires React Query + theme + auth
with sensible test defaults.

## Smoke tests (before merging anything)

### "Build a new product with no fork" test

If someone has only the published crates and packages, can they build
a working product — a GitHub reporting tool, a dashboard, an internal
admin — without cloning this repo or path-dep'ing it? If your change
makes the answer "no", it's wrong.

### "Swap REST for gRPC" test

Pick a handler. If swapping its transport would require rewriting
anything other than the route wiring and DTO shaping, domain logic
leaked into transport. Move it.

### "Drop Postgres for SQLite" test

A consumer running on a laptop should be able to flip a feature flag
and ship the same app on SQLite. If anything outside `starter-store-*`
or the consumer's own repository layer contains backend-specific
knowledge, that's the leak.

### "Custom query without forking" test

A consumer needs a 30-field reporting query joining four tables. They
can write the SQL against the exposed pool inside their own crate,
keep starter on as a normal dependency, and never touch the starter
repo. If your change forces them to either fork or vendor their SQL
into starter, R4 has slipped.

### "Headless appliance" test

A consumer building an edge device should be able to compile a binary
with `starter-auth-token` + `starter-secrets-file` + `starter-ai`
(provider-anthropic only) and **without** pulling
`starter-auth-users`, `starter-secrets-keyring`, `provider-claude`, or
the OpenAI/Codex/Copilot deps. If any of those bleed in through a
non-optional path, R5 has slipped.

### "Block author" test

If a consumer wants to ship a UI extension that runs against a
starter-based server, can they do it with only `starter-client-ts` +
`starter-ui-kit` + `starter-ui-core` from npm? If they have to reach
into a Studio-style consumer's source, the curated facade is missing.

## Non-goals

- Not a framework. There is no "starter app" that consumers extend by
  inheritance or by editing a generated tree. Each crate is a normal
  Rust dependency.
- Not opinionated about the consumer's domain. There is no `User`,
  `Project`, `Tenant`, or `Workflow` type baked in. The consumer owns
  their domain; starter owns the plumbing.
- Not multi-tenant. A consumer that needs multi-tenancy adds it; the
  default is single trust boundary per deployment.
- Not an SSO/OIDC provider. starter ships the `Authenticator` trait
  plus two optional default impls — `starter-auth-token` (single
  owner-token, headless deployments) and `starter-auth-users` (local
  users, sessions, API tokens, roles). Consumers wanting Zitadel /
  Clerk / Keycloak / OAuth social login swap the `Authenticator` impl
  behind the trait — the seam doesn't move.
- Not a workflow / job-queue engine. If a consumer needs background
  jobs, they bring their own.
- Not an AI orchestration layer. `starter-ai` ships the unified
  provider seam (Claude / Codex / Copilot CLIs, Anthropic / OpenAI
  REST) — streaming, cancellation, registry. It does **not** ship
  prompt templates, retry policy, rate limiting, provider failover,
  RAG, or agent loops. Consumers wire `ai-ui` or another library on
  top for the AI domain shape; `starter-ai` is the transport, not the
  brain.
- Not a secret manager. `starter-secrets-*` ships local `SecretStore`
  impls (OS keyring, age-encrypted file). It does **not** ship HSM,
  cloud KMS, secret rotation policy, or audit logging. Consumers
  needing those write a `SecretStore` impl behind the trait.

## Decisions made (previously open questions)

- **Config library:** `figment`. More flexible layering, mature.
- **OpenAPI generator:** `utoipa`. Derive-based, mature, already
  threaded through R7's codegen story.
- **TS state management:** `@tanstack/react-query` for server state +
  `zustand` for client state.
- **MCP transport:** stdio only for v1. SSE/HTTP transports added when
  a consumer needs them. (HTTP transport landed in Phase 5 behind
  `feature = "http"`; SSE still deferred.)
- **gRPC:** `starter-grpc` ships the gRPC sibling of `starter-mcp` —
  one `Tools` service (`ListTools` + unary `CallTool`) backed by the
  same `Tool` trait + `Authenticator` seam. JSON-over-gRPC wire
  envelope; typed per-extension tonic services remain a consumer
  responsibility (see `examples/notes/src/grpc.rs`). The extension
  workspace's `starter-ext-grpc` adapter rides on top to surface
  `contributes.grpc` entries.
- **Theme tokens:** ship a neutral default set in `starter-ui-kit`;
  document the Tailwind-preset override path.
- **SSE shape:** use `axum::response::sse` directly with thin helpers;
  no higher-level abstraction at this layer (consumer-specific stream
  schemas belong in consumer crates).
- **`Repository<T>` derive scope:** deferred to v0.2. Hand-written sqlx
  queries are fine at current volume; the macro carries permanent
  maintenance / debugging cost without a consumer to design against.
  When it lands, scope is bare CRUD + paging + optimistic locking,
  nothing more.
- **Migration source registration API:** fluent builder. Final shape is
  `migrate(pool).with_source("starter", STARTER_SOURCE).with_source(...)
  .run().await`. Each source lands in its own `_sqlx_migrations_<name>`
  table with checksum + version tracking.
- **`Authenticator` signature:** stays `verify(&str)`. Rationale on the
  trait docs at `crates/starter-spi/src/auth/authenticator.rs`. `&Parts`
  and `AuthContext` were both rejected — `&str` keeps the trait useful
  from every transport (HTTP, MCP, future gRPC), and the HTTP boundary
  pre-parses bearer + session cookie into a single string inside
  `starter_server::auth::with_principal`.
- **TS codegen tool:** `openapi-typescript`. Type-only output keeps
  `starter-client-ts` thin; hooks live in `starter-ui-core`. Codegen
  source-of-truth is the checked-in `openapi.json` snapshot generated
  by a Rust test; CI's `openapi-drift` job fails on any diff.
- **`SecretStore` sync vs async:** stays **sync**. Both shipped impls
  (keyring, age-file) are sync at the bottom; secrets read at startup,
  not on the request hot path. Future network-backed impls cache
  aggressively + use `block_in_place` on cold paths, OR ship a sibling
  `AsyncSecretStore`.
- **`starter-ai` provider pinning:** `claude-wrapper` pinned at
  `=0.5.1` (stream-json isn't a stable API). Canary CI lives in a
  separate `starter-ai-canary` repo so a red canary doesn't gate
  normal PRs. The canary repo itself is provisioning work for the
  pre-release pass; recommendation captured in
  `crates/starter-ai/src/lib.rs`.
- **`starter-ai` provider lift mechanics:** clean lift from
  `codeless-workspace/ai-runner`. Source of truth is `starter-ai` from
  the lift forward; the original `ai-runner` can keep running until
  codeless migrates. Decided: dual-track maintenance always rots one
  side.

## Open questions

All open questions from earlier drafts have been resolved (see the
section above). New questions opened during implementation:

- **`Arc<dyn Authenticator>` ergonomics — resolved 2026-05-20.** The
  generic-bound APIs (`with_principal`, `router_with_auth`) now bound
  `A` as `Authenticator + ?Sized`, so callers pass `Arc<dyn
  Authenticator>` directly. The `BoxedAuthenticator` newtype that the
  peer review flagged in `examples/notes/src/server.rs` is gone.
  `McpHttpOptions::with_auth` already accepted `Arc<dyn Authenticator>`
  and was untouched.
- Per-crate questions live in the crate's own `lib.rs` doc when they
  exist.

## Bottom line

**Small libraries, clean dep arrows, opt-in features. A new product
imports the pieces it needs and owns its domain — starter never
becomes the consumer's parent project.**
