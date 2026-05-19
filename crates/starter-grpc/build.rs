//! Compile the `starter.tools.v1` proto into Rust server + client
//! stubs. Both halves are built so the in-crate integration tests can
//! drive the server through a real channel without pulling a second
//! tonic dep in.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = tonic_build::configure().build_server(true).build_client(true);

    // Emit the binary FileDescriptorSet so the optional `reflection`
    // feature can register it with `tonic-reflection`. Always written
    // (cheap, ~few KB); only loaded at runtime when the feature is on.
    let out_dir = std::env::var("OUT_DIR")?;
    let descriptor_path = std::path::PathBuf::from(out_dir).join("starter_tools_v1.bin");
    cfg = cfg.file_descriptor_set_path(&descriptor_path);

    cfg.compile_protos(&["proto/starter.tools.v1.proto"], &["proto"])?;
    Ok(())
}
