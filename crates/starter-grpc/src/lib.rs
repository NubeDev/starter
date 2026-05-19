//! # starter-grpc
//!
//! gRPC server scaffold built on [`tonic`]. The gRPC sibling of
//! [`starter-mcp`](../starter_mcp/index.html): the consumer registers
//! [`starter_spi::tool::Tool`] implementations on a [`ToolRegistry`],
//! and this crate surfaces them as the canonical `starter.tools.v1.Tools`
//! gRPC service — `ListTools` + `CallTool`.
//!
//! ## What this crate is, and isn't
//!
//! - **Is:** a thin, reusable gRPC adapter for the `Tool` surface +
//!   the `Authenticator` seam. Same trust model as `starter-mcp`'s
//!   HTTP transport: optional bearer-token check via
//!   [`starter_spi::auth::Authenticator`], else open.
//! - **Isn't:** a consumer's gRPC API. Consumers with their own
//!   protobuf service should stand up a sibling `tonic::transport::Server`
//!   exactly as `examples/notes/src/grpc.rs` does — `starter-grpc` is
//!   one of the services that goes on it, not the whole router.
//!
//! ## Quick start
//!
//! ```ignore
//! use std::sync::Arc;
//! use starter_grpc::{tools_server, ToolRegistry, GrpcAuth};
//!
//! let registry = Arc::new(ToolRegistry::new().register(my_tool));
//! let server   = tools_server(registry, GrpcAuth::Open);
//!
//! tonic::transport::Server::builder()
//!     .add_service(server)
//!     .serve("0.0.0.0:50051".parse().unwrap())
//!     .await?;
//! ```
//!
//! Pair with [`GrpcAuth::Bearer`] to require a `Authorization: Bearer …`
//! metadata header on every RPC.
//!
//! ## Streaming
//!
//! v0.1 ships unary `CallTool` only — `Tool::invoke` is itself unary.
//! A streaming variant lands in v0.2 when the parent
//! `starter-spi::tool` adds a `StreamingTool` trait; on the wire it
//! will surface as an additive server-streaming RPC under the same
//! `starter.tools.v1.Tools` service.
//!
//! ## Reflection
//!
//! Enable the `reflection` cargo feature and call
//! [`reflection_service`] to register the proto descriptor with
//! `tonic-reflection`, so `grpcurl -list` works without a copy of
//! the `.proto` file. Off by default — most server-to-server
//! integrations have the proto checked in on both sides.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod auth;
mod registry;
mod service;

/// Generated tonic stubs for the `starter.tools.v1` package. Re-exported
/// so consumers building their own clients don't pull a second
/// `tonic_build` invocation.
pub mod proto {
    #![allow(missing_docs, clippy::all)]
    tonic::include_proto!("starter.tools.v1");

    /// `FileDescriptorSet` bytes — used by the optional `reflection`
    /// feature to register the proto with `tonic-reflection`.
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/starter_tools_v1.bin"));
}

pub use auth::GrpcAuth;
pub use registry::ToolRegistry;
pub use service::{tools_server, ToolsService};

#[cfg(feature = "reflection")]
mod reflection;

#[cfg(feature = "reflection")]
pub use reflection::reflection_service;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
