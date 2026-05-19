fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        // Client is built so the integration test can call the gRPC
        // service from the same crate. A pure-consumer build that
        // only serves gRPC could flip this off.
        .build_client(true)
        .compile_protos(&["proto/notes.proto"], &["proto"])?;
    Ok(())
}
