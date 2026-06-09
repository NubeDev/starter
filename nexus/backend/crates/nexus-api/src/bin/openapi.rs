//! Emit the Nexus OpenAPI document to a file (default `openapi.json`).
//!
//! Run as `cargo run --bin nexus-openapi -- [path]`. The frontend codegens its
//! typed client from this artifact, so it is committed and kept add-only. The
//! document is assembled by `nexus_api::openapi::ApiDoc`, which merges the
//! schema surface from `nexus-spi` with the route paths declared in
//! `nexus-api`'s handlers.

use std::path::PathBuf;

use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("openapi.json"));

    let doc = nexus_api::openapi::ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&doc)?;
    std::fs::write(&out, json + "\n")?;
    eprintln!("wrote {}", out.display());
    Ok(())
}
