//! Regenerate the committed JSON Schema artifacts in
//! `crates/starter-ui-ir/schema/` from the live Rust types.
//!
//! Run from the workspace root:
//!
//! ```text
//! cargo run -p starter-ui-ir --bin emit_schema
//! ```
//!
//! The committed artifacts are what downstream consumers (notably
//! `starter-ai-builder-prompt`, which blocks on this file per the
//! job model) read; `tests/schema_artifact.rs` verifies the on-disk
//! file matches the emitter output, so any IR change without a
//! regenerate fails CI.

use std::path::PathBuf;

use starter_ui_ir::schema::{
    emit_action_request_schema, emit_action_response_schema, emit_tree_schema,
};

fn main() -> std::io::Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_dir = manifest_dir.join("schema");
    std::fs::create_dir_all(&schema_dir)?;

    for (name, body) in [
        ("starter-ui-ir.schema.json", emit_tree_schema()),
        (
            "starter-ui-ir.action-request.schema.json",
            emit_action_request_schema(),
        ),
        (
            "starter-ui-ir.action-response.schema.json",
            emit_action_response_schema(),
        ),
    ] {
        let path = schema_dir.join(name);
        std::fs::write(&path, body)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
