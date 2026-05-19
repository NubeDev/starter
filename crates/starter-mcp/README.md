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

## Features

- `http` — exposes `mcp_router` + `McpHttpOptions`. Pulls `axum` +
  `http`. Off by default — pure-stdio consumers don't need them.
- `testing` — in-memory transport pair for round-trip tests.
