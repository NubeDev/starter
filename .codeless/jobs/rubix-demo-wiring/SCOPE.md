# Scope — rubix-demo-wiring

The authoritative design lives at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
§"Success criterion". Latest landed state is captured in the most
recent session handoff under
[`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/).
Where this disagrees with either, **the source wins** — fix this
file rather than diverge.

## Goal

Take the rubix backend from "all five thin-slice PRs are merged
on master at `912667e` but the binary's `main.rs` discards the
MCP router and mounts no auth router" to "the six-step manual
smoke in THIN-SLICE.md §Success criterion passes end to end, by
hand, in under five minutes, against a real Postgres + ClickHouse
running locally."

The architecture is built. This job is **binary wiring + runtime
deps + Claude Desktop integration** — pure glue, no new business
logic.

Three blocks delivered as three PRs against the
`codeless/rubix-demo-wiring` branch.

## What is already landed (do not redo)

These are on master at `912667e` or earlier from PR #27. Re-doing
them creates conflicts.

| What | Where on master |
|---|---|
| MCP exposure via `FlowAsTool::from_registry` | commit `c6f457d` (PR 27) |
| MCP `_meta.acceptLanguage` task-local plumbing (U1 upstream) | landed pre-PR-27 |
| `starter-mcp::InMemoryTransport` for round-trip tests (U2 upstream) | landed pre-PR-27 |
| `FlowRegistry::resolve` + `FlowAsTool::from_registry` (U3 upstream) | landed pre-PR-27 |
| ClickHouse history migration `0002_history/{up,down}.sql` | commit `90facb5` |
| `system_disk_history` write hook in `rubix-tools::system::disk::probe` | commit `90facb5` |
| `rubix-tools::system::alert_send` (log-line impl) + hardcoded `disk_used > 90` insights check | commit `90facb5` |
| Per-tool REST handler at `POST /api/v1/tools/{tool_id}` (`routes::tools::router`, handler ≤20 lines) | commit `e0bb855` |
| `rubix-admin system disk [--json]` CLI verb | commit `e0bb855` |
| `bootstrap_user.rs` admin verb | landed pre-PR-27 |
| Three integration tests (mcp_disk_test, rest_disk_test, cli_disk_test) | PR 27 |
| Bundled flow `com.rubix.scheduled-system-check` | landed earlier |
| EN + ES catalogues for `rubix.system.disk.*` + `rubix.system.alert.sent` | landed earlier |

## What is already answered (do not re-litigate)

| # | Question | Answer locked |
|---|---|---|
| T1 | Recorded-LLM harness | `starter-server::testing` record-and-replay. |
| T2 | Bootstrap operator | First-run claim via `rubix-admin bootstrap-user` (idempotent). |
| T3 | ClickHouse migration runner | `starter-store-clickhouse::MigrationRunner`. |
| T4 | Insights rule wire format | Hardcoded Rust rule for v0. |
| Q6 | `rubix-client` justification | Defer. Stays as a stub. |
| MCP locale | How MCP gets caller locale | `_meta.acceptLanguage` → task-local → `current_locale()` (U1). |
| MCP `FlowAsTool` wiring | How to expose a flow | `FlowAsTool::from_registry`. |

## The current binary gap (concrete)

[`rubix/crates/rubix-agent/src/main.rs`](../../../rubix/crates/rubix-agent/src/main.rs:55)
holds the gap in one line:

```rust
let _mcp_router = mcp.router;   // ← leading underscore: discarded
```

…and never merges an auth router or wraps the tools router in an
authz gate. The README's six-step demo therefore fails at:

- **Step 2** (login) — no auth route mounted.
- **Step 4** (Claude Desktop MCP) — `_mcp_router` thrown away.
- **Step 5** (audit trail) — no changelog write middleware.
- **Step 6** (ClickHouse history) — write hook lives in the disk
  tool but the `ChClient` is not threaded through the registry.

This job closes those four gaps plus the runtime-deps + Claude
Desktop integration that make the demo actually runnable.

## In scope (three blocks)

### Block A — binary wiring

Make `rubix/crates/rubix-agent/src/main.rs` (and the supporting
verb files under `boot/`, `routes/`, possibly a new `auth/`
module) compose every layer that is already implemented somewhere
but currently not reachable from a `cargo run -p rubix-agent`.

Concretely, end-to-end:

1. **Mount the MCP router.** Stop discarding `_mcp_router`. Merge
   it into `app` at `/api/v1/mcp` (HTTP MCP transport — stdio is
   block C). One line of `app.merge(mcp.router)` plus the right
   `State` plumbing.
2. **Mount the auth router** from `starter_auth_users::routes::auth_router`
   (verified shape at `crates/starter-auth-users/src/routes/router.rs:46`).
   The exemplar is `examples/authz-demo/src/main.rs`. Routes land
   under `/api/v1/auth/{login,logout,refresh,me}`.
3. **Wrap `routes::tools::router` in the authz gate** so
   unauthenticated calls return 401 and authenticated calls
   without the matching `REQUIRED_PERMISSION` return 403. The
   `authz_gate_test.rs` integration test must start passing.
4. **Wire `starter-config`** with layered loader (defaults < file
   < env < flags) for: `RUBIX_BIND`, `DATABASE_URL`
   (Postgres), `CLICKHOUSE_URL`, `RUBIX_SECRETS_PATH` (file-backed
   `starter-secrets-file`). Replace today's
   `std::env::var("RUBIX_BIND")` site at
   [`main.rs:60`](../../../rubix/crates/rubix-agent/src/main.rs#L60).
   Default config file path: `$XDG_CONFIG_HOME/rubix/agent.toml`,
   falling back to `~/.config/rubix/agent.toml`.
5. **Wire the `ChClient` into the tool registry state** so the
   disk-tool history write actually has somewhere to write. Today
   `registry::build_tool_registry()` returns
   `Vec<Arc<dyn Tool>>` with no client; the registry needs to
   carry a `ChClient` (or the disk tool needs a constructor that
   accepts one and is invoked from `boot::clickhouse`).
6. **Add a changelog middleware** in front of `routes::tools::router`
   that writes one `starter-changelog` row per authenticated tool
   call (actor = principal, action = tool id, kind = "tool.invoke",
   payload = redacted input). Reuses
   `starter-changelog-postgres::PgChangelogStore` already landed.
7. **Update `docs/design/agent/README.md`** so the wiring picture
   matches what `main.rs` actually does.

Files touched (verb-per-file per FILE-LAYOUT):
- `rubix/crates/rubix-agent/src/main.rs` (the composition root)
- `rubix/crates/rubix-agent/src/boot/config.rs` (new — `starter-config` loader)
- `rubix/crates/rubix-agent/src/boot/auth.rs` (new — `AuthState` construction)
- `rubix/crates/rubix-agent/src/boot/clickhouse.rs` (extend if needed — pass `ChClient` through)
- `rubix/crates/rubix-agent/src/routes/auth.rs` (new — barrel that re-exports `starter_auth_users::routes::auth_router` with rubix-agent's `AppState`)
- `rubix/crates/rubix-agent/src/middleware/changelog.rs` (new — per-request changelog write)
- `rubix/crates/rubix-agent/src/registry.rs` (extend — carry `ChClient`)
- `rubix/docs/design/agent/README.md` (re-write the wiring section present-tense)

### Block B — runtime dependencies

Make `mani run run` work on a fresh dev machine. Today the binary
panics or no-ops on missing Postgres + ClickHouse.

1. **Add `rubix/docker/docker-compose.dev.yaml`** (matches the
   `starter/docker/*` convention) bringing up:
   - Postgres 16 on `127.0.0.1:5433` with database `rubix`,
     user `rubix`, password `rubix-dev`.
   - ClickHouse on `127.0.0.1:8124` (HTTP) + `9001` (native)
     with default user + database `rubix`.
   - Named volumes so data persists across `docker compose down`.
   - No exposed mosquitto / no other services — keep the compose
     file small.
2. **Add a `dev-deps` task** to `rubix/mani.yaml`:
   ```
   dev-deps:
     desc: Bring up Postgres + ClickHouse for local rubix-agent.
     cmd: docker compose -f rubix/docker/docker-compose.dev.yaml up -d
   dev-deps-down:
     desc: Stop and remove local Postgres + ClickHouse.
     cmd: docker compose -f rubix/docker/docker-compose.dev.yaml down
   ```
3. **Add `rubix/dev/agent.toml`** — a sample `starter-config` file
   matching the compose URLs above. Documented as
   `cp rubix/dev/agent.toml ~/.config/rubix/agent.toml` in the
   README.
4. **Rewrite the existing `run` task** in `rubix/mani.yaml` to
   point at the dev config and bind on `127.0.0.1:8088`:
   ```
   run:
     desc: Boot the rubix-agent against local Postgres + ClickHouse.
     cmd: RUBIX_CONFIG=rubix/dev/agent.toml cargo run -p rubix-agent
   ```
5. **Add a `bootstrap` task** that runs `rubix-admin bootstrap-user`
   idempotently before the first `run`:
   ```
   bootstrap:
     desc: Create the bootstrap operator account.
     cmd: cargo run -p rubix-agent --bin rubix-admin -- bootstrap-user --email op@example.com --password rubix-dev
   ```
6. **Add a `demo` aggregate task** that runs `dev-deps`, waits for
   the containers to be ready, runs `bootstrap`, then `run`.
   Useful for the README's "first boot" line:
   `mani run demo`.

Files touched:
- `rubix/docker/docker-compose.dev.yaml` (new)
- `rubix/dev/agent.toml` (new)
- `rubix/mani.yaml` (extend with the four new tasks)
- `rubix/README.md` (add a "Local demo" section pointing at
  `mani run demo` + the six-step smoke from THIN-SLICE.md)

### Block C — Claude Desktop MCP integration

The HTTP MCP transport from Block A is enough for `curl` to
exercise step 4. Claude Desktop wants **stdio MCP**, not HTTP.

1. **Add `rubix-admin mcp` subcommand** that runs the rubix-agent
   in stdio MCP mode: reads JSON-RPC frames on stdin, writes
   responses on stdout, exits when stdin closes. Body lives in
   `rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs`
   (verb-per-file).
2. **Mount the same `FlowAsTool` registry** that the HTTP MCP
   transport uses, behind `starter-mcp`'s stdio loop. Locale
   comes from `_meta.acceptLanguage` per the U1 contract; the
   `LANG` env var is the fallback when the field is missing.
3. **Auth in stdio MCP** — there is no HTTP session. Resolve via
   a pre-authenticated principal: read `RUBIX_PRINCIPAL_EMAIL`
   from the process env at startup, look it up via
   `starter-auth-users::PgUserStore`, fail with a clear error if
   it doesn't exist or is disabled. The session for the launched
   child process inherits the operator the env var named.
4. **Add `rubix/dev/claude-desktop.example.json`** — a snippet
   the user can paste into Claude Desktop's
   `claude_desktop_config.json` to register rubix as an MCP
   server. Names the env vars (`RUBIX_CONFIG`,
   `RUBIX_PRINCIPAL_EMAIL`) and the launch command
   (`rubix-admin mcp`).
5. **Add a `mani run mcp-stdio` task** for ad-hoc local testing:
   ```
   mcp-stdio:
     desc: Run rubix-agent in stdio MCP mode (for Claude Desktop).
     cmd: RUBIX_PRINCIPAL_EMAIL=op@example.com cargo run -p rubix-agent --bin rubix-admin -- mcp
   ```
6. **A new integration test** at
   `rubix/crates/rubix-agent/tests/mcp_stdio_test.rs` that spawns
   `rubix-admin mcp` as a subprocess, writes a `tools/call` frame
   for `com.rubix.scheduled-system-check` with
   `_meta.acceptLanguage: es-AR`, reads the response, asserts
   Spanish output with the right timezone. Uses
   `starter_mcp::testing` if the harness supports child-process
   transport; otherwise hand-rolled stdin/stdout pipes.
7. **Update `docs/design/agent/README.md`** to document the
   stdio MCP path alongside the HTTP one. Update
   `docs/design/i18n-prefs/README.md` only if the stdio locale
   fallback shape (LANG → `_meta.acceptLanguage` → `en`) needs
   to be added to the locale-source list.

Files touched:
- `rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/mod.rs` (new — barrel)
- `rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs` (new — stdio loop)
- `rubix/crates/rubix-agent/src/bin/rubix_admin/main.rs` (extend — register `mcp` subcommand)
- `rubix/crates/rubix-agent/tests/mcp_stdio_test.rs` (new)
- `rubix/dev/claude-desktop.example.json` (new)
- `rubix/mani.yaml` (one new task)
- `rubix/docs/design/agent/README.md` (extend)

## Out of scope (explicit carve-outs)

- **OAuth.** Local password only (matches the demo).
- **Tenants UI / tenant-admin tools.** The bootstrap user belongs
  to the default tenant; no per-tenant operator surface in this
  job.
- **gRPC.** Not in the thin slice.
- **Slack / Telegram / email sinks for `rubix.system.alert_send`.**
  Log-line only.
- **`rubix-client` crate.** Q6 deferred; stays as a stub. The CLI
  uses `probe()` in-process; integration tests use `reqwest`
  directly if needed.
- **Dashboards / flow programmer / analytics report tools.**
  Per-goal broadening is post-demo.
- **Cron triggering** on the bundled flow. GAPS #16.
- **Promotion of the hardcoded insights rule to `rule.rhai`.**
  T4 locked; promotion is a separate job once a second rule
  appears.
- **Removing the dev-dep pin in
  `starter-mcp/Cargo.toml:37-42`.** Phase 2c fix is a separate
  job (the `starter-i18n` interpolate latent bug from commit
  `f7b69fd`).
- **Production hardening.** No TLS, no reverse proxy, no
  hardened secrets, no production Compose. This is a developer-
  laptop demo.
- **Mosquitto / extensions.** Not in the demo.
- **Touching `rubix-old/`.** Read for archaeological context
  only; never copy code.

## Acceptance — when each block is "done"

### Block A — binary wiring
- `cargo build -p rubix-agent` green.
- `cargo test -p rubix-agent` green, including the
  `authz_gate_test.rs` that previously didn't reach the gate.
- A new `cargo test -p rubix-agent --test changelog_middleware_test`
  asserts one changelog row per authenticated tool call.
- `main.rs` does NOT contain `let _mcp_router` (or any
  `let _ =` discarding a router); the MCP router is merged.
- `main.rs` mounts the auth router, the tools router (gated),
  and the MCP router under a single `axum::Router`.
- `boot::config::load()` loads from the four layers
  (defaults < file < env < flags); a test in
  `boot/config.rs` asserts the precedence.
- `docs/design/agent/README.md` describes the new wiring shape
  present-tense — no "Phase X", no "will land", no
  "currently throws away".

### Block B — runtime dependencies
- `mani run dev-deps` brings up Postgres + ClickHouse with no
  manual `docker pull` or config edit.
- `mani run bootstrap` is idempotent (second run is a no-op,
  exit 0).
- `mani run demo` runs `dev-deps`, waits ≤30s for containers
  to be healthy, then `bootstrap`, then `run`. The agent's
  startup log line shows
  `tools>0 mcp_tools>0 flows>0 i18n_keys>0 migrations>0`.
- `rubix/dev/agent.toml` parses cleanly via
  `boot::config::load()` (covered by Block A's config test).
- README "Local demo" section includes the bash from
  THIN-SLICE.md §"Success criterion" verbatim, runnable as-is.

### Block C — Claude Desktop MCP
- `rubix-admin mcp` over stdio responds to a `tools/list` JSON-RPC
  request listing `com.rubix.scheduled-system-check`.
- `cargo test -p rubix-agent --test mcp_stdio_test` passes for
  both `LANG=en_US.UTF-8` and `LANG=es_AR.UTF-8` (or the
  test rolls its own `_meta.acceptLanguage` values, whichever
  is cleaner).
- `RUBIX_PRINCIPAL_EMAIL=missing@example.com rubix-admin mcp`
  exits non-zero with a localised `Diagnostic` saying the user
  doesn't exist (no panic).
- `rubix/dev/claude-desktop.example.json` is valid JSON and
  parseable.
- `mani run mcp-stdio` runs locally without error (manual; the
  binary takes over stdin/stdout, so the verify step is "exit 0
  on SIGINT").

## Hard rules (subset that bites this job)

All from rubix `HOW-TO-CODE.md`, `FILE-LAYOUT.md`, `SCOPE.md`.

- **One verb per file**, ≤400 lines hard, ~100 typical. `mod.rs`
  is a barrel only.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only. Never `SCOPE.md`,
  `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`,
  `docs/scope/`, or `docs/sessions/`.
  `./rubix/scripts/lint-doc-refs.sh` enforces this — run it
  before considering a stage done.
- **No phasing markers** in code: no `// Phase 0`, `// STAGE-1
  done`, `// FIXED:`, `// Previously this used X`.
- **No emojis, no ASCII banners.** `// TODO(name): ...` or
  `// TODO(upstream: <issue>): ...`. Never bare TODOs.
- **Tool outputs are `Diagnostic` + structured data.** Don't
  silently format strings in handlers.
- **Catalogue files are the source of truth** for `MessageKey`
  entries. Adding a key in Rust without matching entries in
  `crates/rubix-spi/catalogues/en.json` AND `es.json` fails
  review.
- **Skill bodies + tool descriptors stay EN canonical.**
- **Layer separation.** REST handlers ≤20 lines; gRPC-swap smoke
  test passes for every handler.
- **Tests live with the code in the same PR.** Unit tests inline.
  Integration tests under `tests/` mirroring source paths.
- **No direct `clickhouse` crate dep** on any rubix crate; pull
  transitively through `starter-store-clickhouse` only.
- **`Done`-doc handover paths must be listed individually** —
  no shell brace expansion (`{a,b}.sql`), no globs (`*.rs`),
  no leading `./`. The runtime's diff-verify pre-check is strict
  and will reject the stage with a misleading `failed` status
  if it can't match the path literally. (See
  [`/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md`](/home/user/code/rust/codeless-workspace/codeless/DOCS/bugs/2026-05-24-diff-verify-brace-expansion.md)
  for the bug report; this workaround stays until the runtime
  is patched.)

## When codeless gets stuck

Codeless cannot ask the human. So the escape hatch is:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever code does compile.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried and why it didn't match
   the spec.
4. Move to the next block only if it does not depend on the
   blocked one. Otherwise stop and wait.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Most recent session handoff: under
  [`/home/user/code/rust/starter/rubix/docs/sessions/`](/home/user/code/rust/starter/rubix/docs/sessions/) —
  read the latest-numbered file.
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
- Exemplars to copy religiously:
  - **Auth wiring**: `/home/user/code/rust/starter/examples/authz-demo/src/main.rs`
    (specifically the `auth_router` + `AuthState` composition and
    the `starter_authz` policy wiring).
  - **`auth_router` API shape**:
    `/home/user/code/rust/starter/crates/starter-auth-users/src/routes/router.rs:46`.
  - **Existing rubix binary glue**:
    `/home/user/code/rust/starter/rubix/crates/rubix-agent/src/main.rs`,
    `boot/*.rs`, `routes/tools.rs`, `registry.rs`.
  - **MCP HTTP transport**: `crates/starter-mcp/src/...`
    (the `mcp_router` constructor already wired by
    `rubix-agent/src/boot/mcp.rs`).
  - **MCP stdio transport**: `crates/starter-mcp/src/...`
    (the existing stdio loop the `rubix-admin mcp` subcommand
    wraps; do NOT reimplement framing).
  - **Compose convention**:
    `/home/user/code/rust/starter/docker/docker-compose.{clickhouse,garage,example}.yml`.
  - **`starter-config` layered loader**: search starter examples
    for usage.
