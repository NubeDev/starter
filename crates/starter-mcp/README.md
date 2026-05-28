# starter-mcp

MCP (Model Context Protocol) server scaffold. Implements `initialize`,
`tools/list`, `tools/call`, and `ping` against a consumer-provided
`ToolRegistry`.

`protocolVersion = "2024-11-05"`. Unknown methods → `-32601`; missing
`name` → `-32602`; tool errors → `-32603`.

## Two transports

### stdio (default)

The MCP norm for desktop tools (Claude Desktop, Codex CLI, …).

```rust
use std::sync::Arc;
use async_trait::async_trait;
use starter_mcp::{run_stdio, ToolRegistry};
use starter_spi::tool::{Tool, ToolDefinition};

struct Hello;

#[async_trait]
impl Tool for Hello {
    fn definition(&self) -> ToolDefinition { /* ... */ }
    async fn invoke(&self, _: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
        Ok(serde_json::json!({ "greeting": "hi" }))
    }
}

let registry = Arc::new(ToolRegistry::new().register(Hello));
run_stdio(registry).await?;
```

### HTTP (`feature = "http"`)

Mounts a `POST /mcp` route alongside the rest of a `starter-server`.

```rust
use std::sync::Arc;
use starter_mcp::{mcp_router, McpHttpOptions, ToolRegistry};

let registry = Arc::new(ToolRegistry::new() /* .register(...) */);

// Open route (single-user / network-isolated):
let router = mcp_router::<AppState>(registry.clone(), McpHttpOptions::new());

// Or bearer-authenticated via any `Authenticator` impl (TokenAuthenticator,
// AuthAuthenticator, JWT, etc.):
let router = mcp_router::<AppState>(
    registry,
    McpHttpOptions::new().with_auth(authenticator),
);

// Merge into the consumer's server:
ServerBuilder::<AppState>::new(state).merge_router(router).build();
```

`POST /mcp` accepts a single JSON-RPC envelope and returns the
response (or `204 No Content` for notifications). On a 401 the body
is a JSON-RPC error frame with code `-32001`.

SSE (Streamable HTTP) progress events are a v0.2 follow-up — the
request/response half is complete today. Tool authors should keep
`Tool::invoke` returning a single `Value`; when SSE lands a sibling
`StreamingTool` trait will opt in to chunked output without breaking
this surface.

## Skills bridge (`feature = "skills"`)

Exposes approved `starter_skills::Skill` bundles as MCP tools so
hosts (Claude Code, Copilot, Codex) can call them. Quarantined
bundles are never registered; revoking an approval takes effect on
the next invoke without restarting the server.

```rust
use starter_mcp::skills_bridge::{register_approved_skills, AddFavoriteTool};
use starter_mcp::ToolRegistry;
use starter_skills::{InMemoryApprovalStore, SkillRegistry};

let skills = SkillRegistry::builder()
    .with_approval_store(InMemoryApprovalStore::new())
    .load_dir("./skills")                                  // repo skills
    .load_dir_quarantined("/var/lib/starter/user-skills")  // user skills
    .build()
    .await?;

let registry = register_approved_skills(ToolRegistry::new(), &skills)
    // Optional: let the LLM mint new (quarantined) favourites.
    .register(AddFavoriteTool::new("/var/lib/starter/user-skills"));

// Hand `registry` to `run_stdio` or `mcp_router` as usual.
```

For a changelog-backed audit row per invoke, implement
`SkillAuditSink` and pass it to `register_approved_skills_with_audit`.
The default sink writes a `tracing::info!` per call.

See [`DOCS/skills-as-mcp-tools.md`](../../DOCS/skills-as-mcp-tools.md)
for the full design.

## Features

- `http` — exposes `mcp_router` + `McpHttpOptions`. Pulls `axum` +
  `http`. Off by default — pure-stdio consumers don't need them.
- `skills` — exposes `skills_bridge` (SkillTool + AddFavoriteTool +
  SkillAuditSink). Pulls `starter-skills`. Off by default.
- `testing` — in-memory transport pair for round-trip tests.
