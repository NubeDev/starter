# gRPC

Overview of the starter workspace's gRPC story: which crates ship, what
they expose on the wire, where authentication plugs in, and how the
extensions kernel surfaces extension-contributed gRPC methods on the
same port.

For the wider transport picture (REST, MCP, CLI, gRPC, workers, UI)
see [`SCOPE.md`](../../../SCOPE.md) and
[`DOCS/extensions/scope/SCOPE.md`](../../extensions/scope/SCOPE.md).

## Two crates, one server

The starter ships **two** gRPC crates. A consumer opts into either,
both, or neither — and any service they add can sit on the same
`tonic::transport::Server::builder()`.

| Crate | Workspace | Surfaces | Service |
|---|---|---|---|
| [`starter-grpc`](../../../crates/starter-grpc/README.md) | parent (`starter`) | `starter_spi::tool::Tool` registry | `starter.tools.v1.Tools` |
| [`starter-ext-grpc`](../../../starter-extensions/crates/starter-ext-grpc/README.md) | `starter-extensions` | every loaded extension's `contributes.grpc[]` | `starter.ext.grpc.v1.ExtensionGrpc` |

Both are feature-gated; neither is reachable by default. A consumer
binary picks them up by depending on the crate and calling the
service-builder helper.

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use starter_grpc::{tools_server, GrpcAuth};
use starter_ext_grpc::{
    build_grpc_methods, extension_grpc_server, BuiltinGrpcDispatcher,
};

let tools = tools_server(tool_registry, GrpcAuth::Bearer(authenticator));

let methods    = build_grpc_methods(&extension_host_registry)?;
let dispatcher = Arc::new(BuiltinGrpcDispatcher::new(handlers));
let ext        = extension_grpc_server(methods, dispatcher, Duration::from_secs(30));

tonic::transport::Server::builder()
    .add_service(tools)
    .add_service(ext)
    .serve("0.0.0.0:50051".parse()?)
    .await?;
```

## `starter-grpc` — the Tool registry as gRPC

Sibling of [`starter-mcp`](../../../crates/starter-mcp/README.md):
same `Tool` trait, same `Authenticator` seam, different transport.

- `ListTools` — enumerate registered tools with their JSON schemas.
- `CallTool` (unary) — invoke a tool by name with a JSON argument
  blob; returns the JSON result.

This crate is the right home for **first-party** RPCs that ride on
the Tool abstraction. It is not a place to put consumer domain APIs;
for those, bring your own tonic `Service` and `add_service` it onto
the same builder (see `examples/notes/src/grpc.rs`).

Features:

- `default` — server + client stubs for `starter.tools.v1.Tools`.
- `reflection` — pulls `tonic-reflection`, so `grpcurl -list <host>`
  works without a copy of the `.proto`.
- `testing` — `TestServer` for in-process integration tests on a
  loopback port.

## `starter-ext-grpc` — extension contributions

Phase 8 adapter in the extensions kernel (SCOPE R13). Surfaces every
`contributes.grpc[]` entry from every loaded extension manifest as
**one backplane service**:

```proto
service ExtensionGrpc {
  rpc ListMethods(ListMethodsRequest)   returns (ListMethodsResponse);
  rpc Invoke(InvokeRequest)             returns (InvokeResponse);
  rpc InvokeStream(InvokeRequest)       returns (stream InvokeEvent);
}
```

Routing is by the manifest-declared `(service, method)` pair. The
arguments and results travel as canonical proto3 JSON strings — the
same envelope the kernel already uses to proxy to process and wasm
extensions, so one codec runs end-to-end.

Typed dynamic per-extension tonic services (`tonic::server::Grpc`
registration with custom codecs) is the v0.2 surface and will land
additively as `starter.ext.grpc.v2`; the v1 backplane stays for
clients that prefer the JSON envelope.

### Dispatcher flavours

Mirrors `starter-ext-cli` exactly:

- **Builtin** — direct in-process call, ships now.
- **Process** — JSON-RPC over stdio to the child, returns
  `UNIMPLEMENTED` in v0.1 with the `request_timeout` knob already
  wired.
- **Wasm** — sandboxed call into the bundle, returns `UNIMPLEMENTED`
  in v0.1.

### Streaming

`InvokeStream` is the streaming entry point. Frames mirror the
kernel's canonical four streaming notifications:

- `stream.event` → one tonic frame per event.
- `stream.end` → graceful end-of-stream.
- `stream.error` → terminal error mapped to a tonic `Status`.
- `stream.cancel` → fired by the adapter when the client disconnects
  (HTTP/2 RST_STREAM, context cancel). Forwarded to the child for
  process / wasm flavours; observed via `ctx.cancel()` for builtin.

This is the same streaming convention REST (SSE / NDJSON), CLI
(line-delimited stdout + SIGINT), and MCP (`notifications/progress`)
follow. The "same source streams over four transports" smoke test
in `starter-ext-smoke` exists to keep that property honest.

## Authentication

`starter-grpc` carries a `GrpcAuth` enum:

- `GrpcAuth::Open` — no auth, suitable for local dev / loopback.
- `GrpcAuth::Bearer(Arc<dyn Authenticator>)` — extracts the
  `authorization: Bearer <token>` metadata, hands it to the
  `Authenticator` trait from `starter-spi`, and either attaches the
  resolved `Principal` to the request extensions or rejects with
  `Status::unauthenticated`.

The `Authenticator` seam is the same one
[`starter-server`](../../../crates/starter-server/README.md) and
[`starter-mcp`](../../../crates/starter-mcp/README.md) use, so a
consumer wires authentication once and every transport honours it.

`starter-ext-grpc` honours capability checks per extension method;
the dispatcher receives the resolved `Principal` and the manifest's
declared capability list and enforces them before the handler runs.

## Status codes

Both crates map domain errors through a small, deliberate set of
tonic `Status` codes:

| Domain | Status |
|---|---|
| Unknown tool / method | `NOT_FOUND` |
| Argument JSON invalid | `INVALID_ARGUMENT` |
| Capability denied | `PERMISSION_DENIED` |
| Auth missing / bad token | `UNAUTHENTICATED` |
| Handler panicked / internal | `INTERNAL` |
| Process / wasm not wired (v0.1) | `UNIMPLEMENTED` |
| Request timeout exceeded | `DEADLINE_EXCEEDED` |
| Stream cancelled by peer | `CANCELLED` |

## Reflection

When the `reflection` feature on `starter-grpc` is enabled, the
server advertises both `starter.tools.v1` and (if `starter-ext-grpc`
is also added) `starter.ext.grpc.v1`, so:

```bash
grpcurl -plaintext localhost:50051 list
# starter.ext.grpc.v1.ExtensionGrpc
# starter.tools.v1.Tools
```

works without any local `.proto` files. Production deployments
typically leave reflection off.

## Versioning

Service identifiers carry a `vN` suffix in the package name
(`starter.tools.v1`, `starter.ext.grpc.v1`). The contract:

- Within a major: only additive changes (new RPCs, new optional
  fields).
- Breaking changes ship as a new package (`v2`) alongside the
  previous version; clients migrate on their own schedule.

The wire layout is what consumers depend on; the Rust API surface of
the two crates is allowed to evolve more freely between starter
releases.

## See also

- [`starter-grpc` crate README](../../../crates/starter-grpc/README.md)
- [`starter-ext-grpc` crate README](../../../starter-extensions/crates/starter-ext-grpc/README.md)
- [Extensions SCOPE](../../extensions/scope/SCOPE.md) — Phase 8 details
- [Parent SCOPE](../../../SCOPE.md) — repo-wide non-goals and rules
- [`starter-mcp`](../../../crates/starter-mcp/README.md) — the MCP
  sibling of `starter-grpc`; same `Tool` trait, different transport
