# Changelog

All notable changes to this workspace's published crates and packages.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/) with workspace-wide
lockstep majors (SCOPE.md §144).

## [Unreleased]

### Security

- Supply-chain audit via `cargo-audit`. New CI job
  `audit` (rustsec/audit-check@v2) gates PRs on real Cargo.lock
  vulnerabilities. Configuration at `.cargo/audit.toml` ignores two
  advisories that don't reach our resolved dependency tree
  (`RUSTSEC-2023-0071` rsa via unused `sqlx-mysql`;
  `RUSTSEC-2025-0111` tokio-tar via dev-deps-only testcontainers) —
  each ignore carries a written rationale.
- `prometheus` bumped 0.13 → 0.14 to retire transitive `protobuf 2.x`
  (`RUSTSEC-2024-0437` uncontrolled-recursion crash).

### Added

- `#[non_exhaustive]` on every public error enum (15 across `spi`,
  `config`, `client-rs`, `cli`, `auth-token`, `auth-users`,
  `secrets-file`). Adding error variants in future releases stays
  additive; consumer match-sites need `_ =>` fallbacks. Cost paid
  in-tree: `starter-server::error::{into_response, status}` now have
  `_ => internal / 500` catch-alls.
- `starter_cli::prompt::password` (wraps `rpassword::prompt_password`)
  and `starter_cli::prompt::confirm` (stdin read, `[y/N]` default-no)
  are now real implementations instead of `todo!()` placeholders.
  Added `rpassword 7` as a workspace dep.
- TS workspace tests via `vitest`. `@nube/starter-client-ts` covers
  `StarterError.fromResponse` (RFC 7807 parsing) and the `/auth/*`
  endpoint wire contracts (8 tests). `@nube/starter-ui-core` covers
  query-key namespacing, mock-server semantics, and an end-to-end
  `<AuthProvider>` flow under jsdom + RTL (16 tests). `pnpm -r test`
  runs in CI alongside typecheck and build.
- `@nube/starter-ui-core/testing` subpath export. Two
  dependency-free helpers:
  - `createMockServer()` — fetch shim covering `/auth/{me,login,logout}`
    with mutable state, plugged into `StarterClient` via the `fetch`
    option.
  - `createAuthWrapper({ client, strategy })` — wrapper component for
    `render(ui, { wrapper })`; sets up `<QueryClientProvider>` +
    `<AuthProvider>`. RTL stays a consumer choice (devDep, not peer).

### Fixed

- `starter_spi::sort::Direction::default()` was a hand-written impl
  the compiler could derive; clippy `-D warnings` was failing
  workspace-wide until the impl was replaced with `#[derive(Default)]`
  + `#[default] Asc`. Caught while validating the Phase 5 exit-criteria
  claim of "clippy --all-features --tests -D warnings clean".

## [0.1.0] — Unreleased

Initial release. See [TODO.md](TODO.md) for the phase-by-phase summary
of what landed. Headline surface:

### Rust

- `starter-spi` — wire types, traits (`Authenticator`, `SecretStore`,
  `AiRunner`), paging primitives. Zero internal deps.
- `starter-store-sqlite` / `starter-store-postgres` — sqlx pool +
  namespaced migration runner + cursor encoding. Each migration
  source lands in its own `_sqlx_migrations_<name>` table.
- `starter-auth-token` — single-owner claim → bearer flow.
- `starter-auth-users` — argon2id passwords, DB-backed sessions,
  API tokens, `/auth/{login,logout,me}` handlers, CSRF protection.
- `starter-secrets-file` (age-encrypted) and
  `starter-secrets-keyring` (OS keyring) — mutually exclusive
  `SecretStore` impls.
- `starter-ai` — unified `AiRunner` over Claude / Codex / Copilot CLI
  wrappers and Anthropic / OpenAI REST. Per-provider features,
  default-off.
- `starter-server` — `ServerBuilder`, `/health`, `/metrics`,
  `/openapi.json`, request-id + latency middleware, `with_principal`
  / `with_role` / `with_scope` guards.
- `starter-mcp` — JSON-RPC dispatch + optional HTTP transport
  (`feature = "http"`) with bearer-auth.
- `starter-cli` — clap building blocks (library, not binary). Ships
  `health` and `openapi` subcommands; consumers wire `serve` /
  `migrate` against their own state.
- `starter-observability` — `init_tracing` guard, `StandardMetrics`,
  request-id constants.
- `starter-config` — figment-based layered loader.
- `starter-client-rs` — Rust HTTP client.

### TypeScript

- `@nube/starter-client-ts` — codegen'd from `openapi.json`;
  `StarterClient` + `/auth/*` endpoint methods + `StarterError`.
- `@nube/starter-ui-core` — `<AuthProvider>` + `useAuth()`,
  `starterQueryKey(...)`, plus `createMockServer` /
  `createAuthWrapper` for tests.
- `@nube/starter-ui-kit` — shadcn/ui components.

### Infrastructure

- CI: rust check / clippy `-D warnings` / test (all-features and
  per-backend); pnpm typecheck / test / build; OpenAPI drift gate
  (snapshot test → TS codegen → `git diff --exit-code`).
- `docker/Dockerfile.template` (two-stage, distroless runtime) and
  `docker/docker-compose.example.yml` (postgres + app with healthcheck).
- README per crate / package; workspace README at-a-glance table.

[Unreleased]: https://github.com/NubeDev/starter/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/NubeDev/starter/releases/tag/v0.1.0
