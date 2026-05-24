# AGENT — how the six goals map onto starter's `ai-agent` node

> **Authoritative spec:** starter's [DOCS/agent/SCOPE.md](../../../DOCS/agent/SCOPE.md).
> Read it first. This doc is the rubix-specific overlay.

## The model

In rubix there is no "agent runtime." Per the SCOPE one-line summary
and R8, the *agent* in rubix is starter's **`ai-agent` node kind**.
Multi-agent orchestration is flow topology. Every rubix goal is a
flow whose root node is an `ai-agent` dispatching tools from the
host's shared `ToolRegistry`, steered by a skill from the host's
shared `SkillRegistry`.

## The six bundled flows

| Goal | Flow id | Skill id | Trigger |
|---|---|---|---|
| 1 — dashboards | `com.rubix.dashboard-assistant` | `com.rubix.dashboard-builder` | explicit |
| 2 — users | `com.rubix.user-admin` | `com.rubix.user-admin` | explicit |
| 3 — flows | `com.rubix.flow-programmer` | `com.rubix.flow-programmer` | explicit |
| 4 — clickhouse | `com.rubix.clickhouse-ruler` | `com.rubix.clickhouse-ruler` | explicit |
| 5 — system | `com.rubix.scheduled-system-check` | `com.rubix.system-checker` | explicit → cron (Phase 4) |
| 6 — analytics | `com.rubix.weekly-report` | `com.rubix.analytics-reporter` | explicit → cron (Phase 4) |

Each lives in [rubix-flows/flows/](../../crates/rubix-flows/flows/) as
a YAML file embedded via `include_dir!`. The matching skill lives
in [rubix-skills/skills/](../../crates/rubix-skills/skills/).

## What the binary wires at boot

`rubix-agent`'s `main.rs` is the wiring composition site. It does,
in this order:

1. Initialises `starter-observability` (tracing + metrics).
2. Loads [`boot::AgentConfig`](../../crates/rubix-agent/src/boot/config.rs)
   through the `starter-config` layered loader:
   defaults → `$XDG_CONFIG_HOME/rubix/agent.toml` → `RUBIX_*` env.
3. Parses the rubix `MessageBundle` from `rubix-spi`.
4. Applies the Postgres migrations (changelog + auth-users) via
   [`boot::apply_migrations`](../../crates/rubix-agent/src/boot/migrations.rs),
   and the ClickHouse `0002_history` migration via
   [`boot::apply_ch_migrations`](../../crates/rubix-agent/src/boot/clickhouse.rs).
   Both steps skip with a warn when their DSN is unset, so the
   binary still boots on a laptop with no databases attached.
5. Builds the `McpSurface` (one `FlowAsTool::from_registry` per
   bundled flow) through
   [`boot::mcp::build_mcp_surface`](../../crates/rubix-agent/src/boot/mcp.rs).
6. Builds the tool registry through
   [`registry::build_tool_registry`](../../crates/rubix-agent/src/registry.rs),
   threading an optional `Arc<ChClient>` into `DiskTool` so the
   disk verb writes one history row per probe whenever a warehouse
   is configured.
7. Composes one `axum::Router` from four sub-routers:

   ```text
   GET  /healthz                              ← health::healthz_router
   POST /api/v1/auth/{login,logout,me}        ← starter_auth_users::routes::auth_router
   POST /api/v1/tools/{tool_id}               ← routes::tools::router
                                                wrapped by middleware::gate_tools
                                                wrapped by middleware::changelog_layer
   POST /api/v1/mcp                           ← starter_mcp::mcp_router (nested under /api/v1)
   ```

   When `RUBIX_DATABASE_URL` is unset the binary warns and serves
   `/healthz`, `/api/v1/mcp`, and an **ungated** `/api/v1/tools/*` —
   the auth + authz + changelog sandwich requires Postgres. The
   production smoke path always sets the DSN.

8. Hands the composed router to
   [`health::serve`](../../crates/rubix-agent/src/health.rs).

## The auth + authz + audit sandwich

The tools router runs inside three concentric middleware layers,
outermost first:

1. **`middleware::changelog_layer`** — writes one
   `starter_changes` row per authenticated dispatch via
   `starter_changelog_postgres::PgChangeRecorder`. Actor =
   `Principal::subject`; resource kind = `tool.invoke`; resource
   id = the tool id from the path; payload = the request body
   with secret-looking top-level keys (`password`, `token`, …)
   redacted. Anonymous requests skip the recorder.
2. **`starter_server::auth::with_principal`** — resolves the
   bearer token / session cookie to a `Principal` and stamps it
   on the request extensions. Missing credentials → `401`.
3. **`starter_authz::with_permission_owned("rubix.tool", "invoke")`** —
   the engine consults the cached policy for the
   `(principal, rubix.tool, invoke)` triple. Deny → `403`.

The engine is built by
[`boot::authz::build_engine`](../../crates/rubix-agent/src/boot/authz.rs)
as a `StaticRbacEngine` over an empty `AuthzConfig`. The
collection-level `rubix.tool:invoke` gate is v0; per-verb
permissions land alongside the rule data (the
`REQUIRED_PERMISSION` constants on each `rubix-spi::dto::system::*`
module are the canonical mapping).

## How the parts compose at boot

```text
NodeKindRegistry  ← starter built-ins (includes ai-agent) + extensions
ToolRegistry      ← rubix-tools + extension-contributed
SkillRegistry     ← rubix-skills (approved) + operator dir + extensions (quarantined)
FlowRegistry      ← rubix-flows + operator dir + extensions

then:
  starter-flow::Engine::builder()
    .with_runner(starter-ai Claude CLI)
    .with_tools(tools)
    .with_skills(skills)
    .with_node_kinds(kinds)
    .with_flows(flows)
    ...
  starter-mcp::mcp_router(engine.as_tool_registry(), ...)  // ← every flow is an MCP tool
  starter-server::ServerBuilder::new(state).merge_router(mcp_router)...
```

Every bundled flow auto-surfaces as an MCP tool via
`FlowAsTool` — see SCOPE R7. No per-flow MCP code.

## What rubix never builds

- A second LLM seam (SCOPE R8 — `AiRunner` only).
- A second tool / skill / flow registry (R7).
- An extension host (`starter-ext-flow` does that — see
  [STARTER-CHANGES.md](./STARTER-CHANGES.md)).
- A scheduler (cron is a flow trigger upstream in `starter-flow`).
