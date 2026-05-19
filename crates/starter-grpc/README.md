# starter-grpc

gRPC server scaffold built on [tonic]. The gRPC sibling of
[`starter-mcp`]: a consumer registers `starter_spi::tool::Tool` impls,
and this crate exposes them as the `starter.tools.v1.Tools` gRPC
service — `ListTools` + `CallTool` (unary).

```rust,ignore
use std::sync::Arc;
use starter_grpc::{tools_server, GrpcAuth, ToolRegistry};

let registry = Arc::new(ToolRegistry::new().register(my_tool));
let service  = tools_server(registry, GrpcAuth::Bearer(my_authenticator));

tonic::transport::Server::builder()
    .add_service(service)
    .serve("0.0.0.0:50051".parse()?)
    .await?;
```

## Features

- `default` — server + client stubs for `starter.tools.v1.Tools`.
- `reflection` — exposes the proto via `tonic-reflection`, so
  `grpcurl -list <host>` works without a `.proto` file.
- `testing` — `testing::TestServer` for in-process integration tests
  bound to a loopback port.

## What this crate is, and isn't

- **Is:** a thin, reusable adapter for the `Tool` registry + the
  `Authenticator` seam. Same trust model as `starter-mcp`'s HTTP
  transport.
- **Isn't:** a consumer's gRPC API. Bring your own tonic service for
  domain RPCs — `examples/notes/src/grpc.rs` shows the pattern. This
  crate hands you the `tools` slice that goes on the same `Server::builder()`.

## Streaming

v0.1 ships unary `CallTool` only. Streaming lands as an additive RPC
under the same v1 service when `starter-spi::tool` grows a
`StreamingTool` trait (v0.2). Extensions that want streaming today
go through [`starter-ext-grpc`] in the sibling `starter-extensions`
workspace.

[tonic]: https://github.com/hyperium/tonic
[`starter-mcp`]: ../starter-mcp/README.md
[`starter-ext-grpc`]: ../../starter-extensions/crates/starter-ext-grpc/README.md
