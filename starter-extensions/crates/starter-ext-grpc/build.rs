//! Compile the `starter.ext.grpc.v1` proto into server + client stubs.
//!
//! v0.1 surfaces `contributes.grpc` entries as a generic backplane —
//! one service with `Invoke` (unary) and `InvokeStream`
//! (server-streaming) methods routed by `(service, method)` strings.
//! The per-extension `.proto` files in each bundle remain the schema
//! contract for clients; the adapter does not perform runtime
//! protobuf reflection in v0.1. A future iteration may add dynamic
//! `tonic::server::Grpc` registration once a consumer needs typed
//! wire frames.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let descriptor_path =
        std::path::PathBuf::from(out_dir).join("starter_ext_grpc_v1.bin");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&["proto/starter.ext.grpc.v1.proto"], &["proto"])?;
    Ok(())
}
