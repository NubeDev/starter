## Done

- Mounted starter-mcp router under `POST /api/v1/mcp` (removed the `let _mcp_router = mcp.router;` discard at main.rs:55).
- Mounted `starter_auth_users::routes::auth_router` under `/api/v1/auth/{login,logout,me}` via a new `routes/auth.rs` barrel; `boot/auth.rs` builds the `AuthState` + `AuthAuthenticator` pair from a Postgres pool using the Pg* stores.
- Wrapped the tools router in `middleware::gate_tools` (sandwich of `starter_server::auth::with_principal` + `starter_authz::with_permission_owned("rubix.tool","invoke")`) so unauth → 401 and authed-without-perm → 403. Engine is a process-local `StaticRbacEngine` built in `boot/authz.rs` (no Postgres required for the gate; rule data is the v0 deferral).
- Added `boot/config.rs` — `AgentConfig` via `starter-config` Loader (defaults → `$XDG_CONFIG_HOME/rubix/agent.toml` → `RUBIX_*` env). Replaces the ad-hoc `std::env::var("RUBIX_BIND")`; also honours legacy `RUBIX_DSN`/`RUBIX_CH_URL` names.
- Extended `registry::build_tool_registry` to take `Option<Arc<ChClient>>`, threading it into `DiskTool::with_history`. Updated `tests/rest_disk_test.rs` for the new signature.
- Added `middleware/changelog.rs` — writes one `starter_changes` row per authenticated tool call via `PgChangeRecorder`, with a coarse secret-key redactor on the payload. New `tests/changelog_middleware_test.rs` proves one-row, anonymous-skips, and redaction guarantees using an in-memory SQLite recorder.
- Rewrote `rubix/docs/design/agent/README.md` in present tense, documenting the four-router composition and the auth+authz+audit sandwich.
- Committed as `stage 1 (block A, binary wiring) — mount MCP + auth + gated tools + audit` (f854fe1) on branch `codeless/rubix-demo-wiring`.

## Next

- Stage 2 (Block B): docker-compose.dev with Postgres + ClickHouse, `rubix/dev/agent.toml` sample, four new mani tasks including `mani run demo`.
- Stage 3 (Block C): `rubix-admin mcp stdio` subcommand, `RUBIX_PRINCIPAL_EMAIL` principal resolution, `dev/claude-desktop.example.json`, `mcp_stdio_test`.

## What you need to know

- `cargo build -p rubix-agent`, `cargo test -p rubix-agent` (all 28 tests pass, 2 ignored), and `./rubix/scripts/lint-doc-refs.sh` are all green.
- The task spec named the changelog type `PgChangelogStore` but the actual API is `starter_changelog_postgres::PgChangeRecorder` (records via `ChangeRecorder::transaction`). I used the real type and mapped task fields onto the `Change` envelope: actor → `Actor::User { subject }`, kind → `resource.kind = "tool.invoke"`, action (tool id) → `resource.id`, op → `Op::Custom("invoke")`, payload → `after` (with secret-key redaction).
- The authz gate is currently collection-level (`rubix.tool:invoke`); per-verb permissions (the `REQUIRED_PERMISSION` constants on `rubix-spi::dto::system::*`) are documented as the canonical mapping but not yet enforced — they activate once policy data lands in `starter_authz_rules`. `AuthzConfig::default()` has `default_policy = true`, so authenticated principals currently pass the gate (the 403 path is wired but inert in v0). The `authz_gate_test.rs` "live" test was already `#[ignore]`-tagged upstream; only the const-declaration test runs and passes.
- When `RUBIX_DATABASE_URL` is unset the binary still boots, serving `/healthz`, `/api/v1/mcp`, and an **ungated** `/api/v1/tools/*` (with a warn log). The production smoke path always sets the DSN, in which case auth + authz + changelog all activate.
- Middleware layer order: changelog wraps the bare tools router first, then `gate_tools` wraps the audited router — so request flow is `with_principal → with_permission → changelog → handler`, ensuring `Principal` is in extensions when the changelog reads it.
- Dependencies added: `starter-changelog`, `starter-authz`, `chrono` (deps); `starter-changelog-sqlite`, `starter-store-sqlite` with `testing` feature (dev-deps).

## Open questions

- (none)
