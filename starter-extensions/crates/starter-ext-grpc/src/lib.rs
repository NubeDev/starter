//! # starter-ext-grpc — Adapter Phase 8 (SCOPE R13)
//!
//! Surfaces every `contributes.grpc` entry across every loaded
//! extension as a tonic backplane service —
//! `starter.ext.grpc.v1.ExtensionGrpc` with `ListMethods`, `Invoke`
//! (unary) and `InvokeStream` (server-streaming) — routed by
//! `(service, method)` pairs declared in each extension's manifest.
//!
//! Sibling crate to `starter-ext-cli`, `starter-ext-server`, and
//! `starter-ext-mcp`: same dispatcher trait shape, same Builtin /
//! Process / Wasm split, same `request_timeout` knob, same streaming
//! convention (kernel `stream.event`/`stream.end`/`stream.cancel`
//! notifications → tonic server-streaming response).
//!
//! Two dispatch flavours ship in v0.1:
//!
//! - [`BuiltinGrpcDispatcher`] — host populates a
//!   [`BuiltinGrpcRegistry`] with one closure per
//!   `(extension, contribute_id)` at startup. Calls run in-process; no
//!   JSON-RPC frame is ever serialised.
//! - [`ProcessGrpcDispatcher`] / [`WasmGrpcDispatcher`] — return
//!   `DispatchError::NotWired` in v0.1, with the `request_timeout`
//!   knob already in the constructor so the wiring shape is uniform
//!   when the synchronous JSON-RPC dispatch slice lands additively.
//!
//! Matching pattern from `starter-ext-cli` / `starter-ext-server::rest`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod adapter;
mod dispatcher;
mod service;

/// Generated tonic stubs for the `starter.ext.grpc.v1` package.
pub mod proto {
    #![allow(missing_docs, clippy::all)]
    tonic::include_proto!("starter.ext.grpc.v1");

    /// `FileDescriptorSet` bytes for the optional reflection wiring
    /// (consumed by `starter-grpc::reflection_service`-style helpers
    /// in downstream binaries).
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/starter_ext_grpc_v1.bin"));
}

pub use adapter::{build_grpc_methods, BuildGrpcError, GrpcMethod};
pub use dispatcher::{
    BuiltinGrpcDispatcher, BuiltinGrpcRegistry, CancelHandle, DispatchError, GrpcDispatcher,
    GrpcHandler, GrpcStreamingHandler, NotWiredGrpcDispatcher, ProcessGrpcDispatcher,
    StreamResponse, WasmGrpcDispatcher, DEFAULT_REQUEST_TIMEOUT,
};
pub use service::{extension_grpc_server, ExtensionGrpcService};

// Re-export the streaming event type so callers building handlers
// don't need to reach into `starter_ext_sdk::ctx` themselves.
pub use starter_ext_sdk::ctx::Event as StreamEvent;
