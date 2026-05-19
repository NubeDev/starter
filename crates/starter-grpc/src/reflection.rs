//! Optional gRPC reflection wiring. Enabled by `feature = "reflection"`.
//!
//! Allows `grpcurl -list <host>` and similar tools to discover the
//! `starter.tools.v1.Tools` service without a local copy of the
//! `.proto`. Off by default — most server-to-server integrations
//! ship the proto on both sides and don't need the extra dependency.

use tonic_reflection::server::{ServerReflection, ServerReflectionServer};

/// Build the reflection service for the `starter.tools.v1` proto.
/// Add it to your `tonic::transport::Server::builder()` alongside
/// [`crate::tools_server`].
pub fn reflection_service() -> ServerReflectionServer<impl ServerReflection> {
    tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(crate::proto::FILE_DESCRIPTOR_SET)
        .build_v1()
        .expect("reflection FileDescriptorSet is generated at build time and is always valid")
}
