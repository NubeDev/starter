//! End-to-end Phase 1 smoke test.
//!
//! Stages the `examples/hello-builtin` bundle into a tempdir, runs the
//! kernel loader, wires the registered tools through the MCP adapter,
//! and dispatches a tool call against the resulting `ToolRegistry`.
//!
//! Also exercises the SCOPE "Bad manifest is isolated to its own
//! extension" smoke test: a second bundle with a deliberately-broken
//! manifest lands as `Failed` while `hello-builtin` keeps working.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_spi::LifecycleState;
use tempfile::tempdir;

/// Path to the `hello-builtin` example bundle, relative to *this* test
/// crate. CARGO_MANIFEST_DIR is `crates/starter-ext-mcp`.
fn hello_bundle_src() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("examples/hello-builtin")
}

/// Copy the bundle's user-facing files (manifest + schemas + docs) into
/// `dest`. The .rs source is not needed at host load time — only the
/// static metadata files the manifest references. SCOPE R7: those files
/// are the only artefact the host reads at runtime.
fn copy_bundle(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    fs::copy(src.join("block.yaml"), dest.join("block.yaml")).unwrap();
    fs::create_dir_all(dest.join("schemas")).unwrap();
    for name in ["echo_in.json", "echo_out.json"] {
        fs::copy(
            src.join("schemas").join(name),
            dest.join("schemas").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(dest.join("docs")).unwrap();
    for name in ["echo.md", "README.md"] {
        fs::copy(src.join("docs").join(name), dest.join("docs").join(name)).unwrap();
    }
}

#[tokio::test]
async fn hello_builtin_is_reachable_through_mcp() {
    let tmp = tempdir().unwrap();
    let bundle_dest = tmp.path().join("com.acme.hello");
    copy_bundle(&hello_bundle_src(), &bundle_dest);

    // Phase 1: scan + validate + commit.
    let records = Loader::scan(tmp.path()).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    assert_eq!(outcome.validated, 1, "the hello bundle must validate");
    assert_eq!(outcome.failed, 0);

    // Adapter validation (R13): the host's BuiltinTable knows about the
    // statically-linked `hello-builtin` crate.
    let mut table = BuiltinTable::new();
    hello_builtin::register(&mut table);
    let builtins = Arc::new(table);

    let (tools, register_outcome, register_result) =
        starter_ext_mcp::register_tools(&registry, &builtins, starter_mcp::ToolRegistry::new());
    register_result.expect("every tool binding wires cleanly");
    assert_eq!(register_outcome.extensions_seen, 1);
    assert_eq!(register_outcome.tools_registered, 1);

    // Dispatch a call exactly as the MCP server would.
    let tool = tools
        .get("com.acme.hello.echo")
        .expect("MCP ToolRegistry surfaces the extension's tool by manifest id");
    let definition = tool.definition();
    assert_eq!(definition.name, "com.acme.hello.echo");
    assert!(
        definition.description.contains("Echo back"),
        "description bytes are read from `description_file` at load time (R7)"
    );

    let input = serde_json::json!({ "message": "hello from the kernel" });
    let output = tool.invoke(input.clone()).await.unwrap();
    assert_eq!(output, input, "echo handler returns input verbatim");
}

#[tokio::test]
async fn bad_manifest_is_isolated_to_its_own_extension() {
    let tmp = tempdir().unwrap();

    // Good: copy the real `hello-builtin` bundle.
    let good_dest = tmp.path().join("com.acme.hello");
    copy_bundle(&hello_bundle_src(), &good_dest);

    // Broken: a bundle whose manifest fails `deny_unknown_fields`.
    let bad_dest = tmp.path().join("com.acme.broken");
    fs::create_dir_all(&bad_dest).unwrap();
    fs::write(
        bad_dest.join("block.yaml"),
        r#"
v: 1
id: com.acme.broken
version: 0.0.1
display_name: "B"
runtime: { kind: builtin, crate_name: b }
nope_unknown_top_level: true
"#,
    )
    .unwrap();

    let records = Loader::scan(tmp.path()).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    assert_eq!(outcome.validated, 1, "good bundle survives the bad sibling");
    assert_eq!(outcome.failed, 1, "broken bundle ends up in Failed");

    let good = registry
        .get_by_id_str("com.acme.hello")
        .expect("good extension is queryable by parsed id");
    assert_eq!(good.state, LifecycleState::Validated);

    // The bad record is still in the registry (keyed by the synthetic
    // `<unparsed:…>` slot when its id never parsed). Find it by
    // id_hint — SCOPE: "failure is isolated to the bad extension's
    // record (state: Failed, with a parseable reason)".
    let bad = registry
        .list()
        .iter()
        .find(|r| r.id_hint == "com.acme.broken")
        .expect("broken record is still surfaced as a diagnostic");
    assert_eq!(bad.state, LifecycleState::Failed);
    assert!(bad.failure.is_some());

    // And the adapter wires only the good extension's tool — the bad
    // bundle does not contaminate `ToolRegistry`.
    let mut table = BuiltinTable::new();
    hello_builtin::register(&mut table);
    let (tools, _outcome, result) = starter_ext_mcp::register_tools(
        &registry,
        &Arc::new(table),
        starter_mcp::ToolRegistry::new(),
    );
    result.expect("adapter wiring succeeds despite the failed sibling");
    assert!(tools.get("com.acme.hello.echo").is_some());
    let names: Vec<_> = tools.list().into_iter().map(|d| d.name).collect();
    assert_eq!(names, vec!["com.acme.hello.echo"]);
}
