# starter-ext-grpc — Adapter Phase 8 (SCOPE R13)

The gRPC transport adapter for the `starter-extensions` kernel. Surfaces
every `contributes.grpc[]` entry across every loaded extension as a
single tonic backplane service —
`starter.ext.grpc.v1.ExtensionGrpc` with `ListMethods`, `Invoke`
(unary) and `InvokeStream` (server-streaming) — routed by the
manifest-declared `(service, method)` pairs.

Sibling crate to `starter-ext-cli`, `starter-ext-server`,
`starter-ext-mcp`, and `starter-ext-workers`. Same dispatcher trait
shape, same Builtin / Process / Wasm split, same `request_timeout`
knob, same streaming convention (kernel `stream.event` notifications
→ tonic server-streaming response, client disconnect →
`stream.cancel`).

## Quick start

```rust,ignore
use std::sync::Arc;
use std::time::Duration;
use starter_ext_grpc::{
    build_grpc_methods, extension_grpc_server,
    BuiltinGrpcDispatcher, BuiltinGrpcRegistry,
};

// At host startup, after the extension loader runs:
let methods = build_grpc_methods(&host_registry)?;

let handlers = BuiltinGrpcRegistry::new()
    .register(weather_id, "com.acme.weather.current", |args, ctx| {
        Ok(serde_json::json!({ "temp_c": 21.4 }))
    });
let dispatcher = Arc::new(BuiltinGrpcDispatcher::new(Arc::new(handlers)));

let service = extension_grpc_server(methods, dispatcher, Duration::from_secs(30));

tonic::transport::Server::builder()
    .add_service(service)
    // Add `starter-grpc::tools_server(...)` here too if the host
    // wants the Tool surface on the same port.
    .serve("0.0.0.0:50051".parse()?)
    .await?;
```

## What v0.1 ships, and what v0.2 will

- **Ships now:** the contract crate, the adapter (collision check,
  description/proto resolution, R7 byte-identical surfacing), the
  dispatcher trait + builtin path, process/wasm stubs returning
  `UNIMPLEMENTED` with the `request_timeout` knob already wired,
  the tonic backplane service with unary + server-streaming
  routing, status code mapping, and client-disconnect cancellation.
- **Lands additively:** dynamic per-extension `tonic::server::Grpc`
  registration (typed proto messages on the wire, not JSON-over-gRPC).
  When it lands it surfaces as a sibling service under
  `starter.ext.grpc.v2`; the v1 backplane stays for clients that
  prefer the JSON envelope.

## Wire shape rationale

The v1 backplane carries arguments and results as canonical proto3
JSON strings, not typed protobuf messages. Reasoning:

- The kernel already speaks JSON when proxying to process / wasm
  extensions; a JSON-over-gRPC backplane keeps **one codec**
  end-to-end.
- Dynamic `tonic::server::Grpc` registration requires a custom codec
  per extension + runtime prost reflection — a sizeable sub-project
  that should land when a real consumer needs typed wire frames.
- The per-extension `.proto` files in each bundle remain the schema
  contract; a client that has the proto serializes/deserializes
  against it client-side and ships the JSON envelope to the
  backplane.

## Streaming

`InvokeStream` server-streaming responses pump kernel `stream.event`
notifications to the client one frame per event. Client disconnect
(HTTP/2 RST_STREAM, context cancel) fires the dispatcher's
`CancelHandle` — for the builtin path the extension's handler observes
cancellation through `ctx.cancel()`; for process / wasm flavours
(v0.2) the cancel becomes a `stream.cancel` JSON-RPC notification to
the child.

## Tests

```bash
cargo test -p starter-ext-grpc
```
