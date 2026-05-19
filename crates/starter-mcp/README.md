# starter-mcp

MCP (Model Context Protocol) dispatcher over stdio. Implements
`initialize`, `tools/list`, `tools/call`, and `ping` against a
consumer-provided `ToolRegistry`.

## Usage

```rust
use starter_mcp::server::{dispatch, ToolRegistry};

let mut tools = ToolRegistry::new();
tools.register("hello", /* fn(JsonValue) -> Result<JsonValue, _> */);

// Wire to your stdio loop:
let response = dispatch(&tools, request_json).await;
```

`protocolVersion = "2024-11-05"`. Unknown methods → `-32601`; missing
`name` → `-32602`; tool errors → `-32603`.

## Auth seam

stdio is single-process so no per-request credential is needed. The
`Authenticator` hook ships with the HTTP / SSE transport (not yet
landed).

No features.
