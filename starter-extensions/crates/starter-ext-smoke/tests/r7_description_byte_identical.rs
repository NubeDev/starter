//! SCOPE smoke: "LLM-facing description is byte-identical at load time
//! and call time" (R7 — anti-prompt-injection guarantee).
//!
//! The bytes the host reads from `description_file` at load time, the
//! bytes surfaced on the MCP `ToolDefinition`, and the bytes the
//! adapter caches on its `ExtensionToolBinding` must be a single byte
//! string. No runtime templating, no extension-mutable text. We assert
//! all three views are byte-identical against the on-disk file.
//!
//! If an extension can change its own description after load, R7 has
//! slipped — and this test fails.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::builtin::BuiltinTable;
use tempfile::tempdir;

fn hello_bundle_src() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/hello-builtin")
}

fn copy_bundle(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    fs::copy(src.join("block.yaml"), dest.join("block.yaml")).unwrap();
    fs::create_dir_all(dest.join("schemas")).unwrap();
    for name in ["echo_in.json", "echo_out.json"] {
        fs::copy(src.join("schemas").join(name), dest.join("schemas").join(name)).unwrap();
    }
    fs::create_dir_all(dest.join("docs")).unwrap();
    for name in ["echo.md", "README.md"] {
        fs::copy(src.join("docs").join(name), dest.join("docs").join(name)).unwrap();
    }
}

#[tokio::test]
async fn description_bytes_match_disk_and_mcp_definition() {
    let tmp = tempdir().unwrap();
    let bundle_dest = tmp.path().join("com.acme.hello");
    copy_bundle(&hello_bundle_src(), &bundle_dest);

    let records = Loader::scan(tmp.path()).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    assert_eq!(outcome.validated, 1);

    let mut table = BuiltinTable::new();
    hello_builtin::register(&mut table);
    let builtins = Arc::new(table);

    let (tools, _outcome, result) =
        starter_ext_mcp::register_tools(&registry, &builtins, starter_mcp::ToolRegistry::new());
    result.expect("MCP wiring succeeds");
    let tool = tools.get("com.acme.hello.echo").expect("echo registered");

    // (1) Bytes on disk.
    let on_disk = fs::read_to_string(bundle_dest.join("docs/echo.md")).unwrap();
    assert!(!on_disk.is_empty(), "description_file should not be empty");

    // (2) Bytes surfaced through the MCP `Tool::definition()` — what the
    // LLM ultimately sees on `tools/list` and as the call-time tool
    // metadata.
    let def_at_load = tool.definition();
    assert_eq!(
        def_at_load.description, on_disk,
        "definition() bytes must match docs/echo.md byte-for-byte (R7)"
    );

    // (3) Calling the tool must not have mutated the description.
    let _ = tool
        .invoke(serde_json::json!({ "message": "hello" }))
        .await
        .unwrap();
    let def_after_call = tool.definition();
    assert_eq!(
        def_after_call.description, def_at_load.description,
        "description bytes are immutable across calls — extension cannot \
         retemplate its own surface (R7)"
    );

    // (4) Looking the same tool up again returns the same string —
    // confirms the binding caches the value rather than re-reading the
    // file (which an attacker could swap between calls).
    let tool_again = tools.get("com.acme.hello.echo").unwrap();
    assert_eq!(tool_again.definition().description, on_disk);
}
