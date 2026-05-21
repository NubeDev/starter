//! Verifies the committed JSON Schema artifacts match the live
//! `schemars` output, and that emission is deterministic across
//! repeated calls.
//!
//! When this test fails the fix is one command:
//!
//! ```text
//! cargo run -p starter-ui-ir --bin emit_schema
//! ```
//!
//! The first assertion guards drift between the source-of-truth Rust
//! types and the artifact downstream consumers
//! (`starter-ai-builder-prompt`, the deferred Flutter codegen) read.
//! The determinism assertion guards the more subtle failure where
//! `schemars` (or `serde_json`) starts ordering keys non-
//! reproducibly — that would invalidate the artifact even when the
//! IR was unchanged.

use std::path::PathBuf;

use starter_ui_ir::schema::{
    emit_action_request_schema, emit_action_response_schema, emit_tree_schema,
};

const ARTIFACTS: &[(&str, fn() -> String)] = &[
    ("starter-ui-ir.schema.json", emit_tree_schema),
    (
        "starter-ui-ir.action-request.schema.json",
        emit_action_request_schema,
    ),
    (
        "starter-ui-ir.action-response.schema.json",
        emit_action_response_schema,
    ),
];

#[test]
fn committed_schema_matches_emitter() {
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema");
    for (name, emit) in ARTIFACTS {
        let on_disk = std::fs::read_to_string(schema_dir.join(name))
            .unwrap_or_else(|e| panic!("missing schema artifact `{name}`: {e}"));
        let fresh = emit();
        assert_eq!(
            on_disk, fresh,
            "schema artifact `{name}` is stale — run \
             `cargo run -p starter-ui-ir --bin emit_schema`",
        );
    }
}

#[test]
fn emission_is_deterministic_across_runs() {
    for (_, emit) in ARTIFACTS {
        let a = emit();
        let b = emit();
        let c = emit();
        assert_eq!(a, b, "schema emission differs run-to-run (run 1 vs run 2)");
        assert_eq!(b, c, "schema emission differs run-to-run (run 2 vs run 3)");
        assert!(a.ends_with('\n'), "schema artifact must end with newline");
    }
}
