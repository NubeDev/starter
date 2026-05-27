//! End-to-end test for the `CapabilityFactory` seam.
//!
//! Builds a tiny builtin extension whose handler tries to call
//! `ctx.warehouse_read().query(...)`. With the default
//! `StubCapabilityFactory` the call refuses with the canonical
//! "not wired" message; with a host-installed factory the call
//! refuses with that factory's marker. That delta is the proof
//! that `BuiltinRestDispatcher::with_capability_factory` actually
//! threads the override down into `build_ctx`.

use std::sync::Arc;

use serde_json::{json, Value};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::builtin::{BuiltinEntry, BuiltinTable};
use starter_ext_sdk::ctx::{EventBusBackend, WarehouseReadBackend};
use starter_ext_server::{
    BuiltinRestDispatcher, CapabilityFactory, DispatchError, RestDispatcher,
};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::warehouse::{Row, TemplateSpec};
use starter_ext_spi::{Error, ExtensionId, Result as ExtResult};
use tempfile::tempdir;

const BUNDLE: &str = r#"
v: 1
id: com.acme.cap
version: 0.1.0
display_name: "Cap"
runtime: { kind: builtin, crate_name: cap_demo }
contributes:
  tools:
    - id: com.acme.cap.read
      input_schema:  schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/read.md
"#;

fn write(root: &std::path::Path, rel: &str, body: &[u8]) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, body).unwrap();
}

fn bundle_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let dir = root.join("cap");
    std::fs::create_dir_all(&dir).unwrap();
    write(&dir, "block.yaml", BUNDLE.as_bytes());
    write(&dir, "docs/read.md", b"# read");
    write(&dir, "schemas/in.json", br#"{ "type": "object" }"#);
    write(&dir, "schemas/out.json", br#"{ "type": "object" }"#);
    (tmp, root)
}

fn load_registry(root: &std::path::Path) -> Arc<ExtensionRegistry> {
    let recs = Loader::scan(root).validate_all();
    let mut reg = ExtensionRegistry::new();
    Loader::commit(recs, &mut reg);
    reg.seal();
    Arc::new(reg)
}

/// Builtin table whose single entry calls `ctx.warehouse_read().query(...)`
/// and surfaces the result (or the error) back to the caller. The
/// handler reports the surfaced kernel-error message via
/// `Error::ExtensionInternal` so the dispatcher renders it as a
/// 500-ish payload the test can read off the success path of
/// `RestDispatcher::dispatch` (after `from_kernel`).
fn read_calling_table() -> Arc<BuiltinTable> {
    let mut table = BuiltinTable::new();
    let ext = ExtensionId::new("com.acme.cap").unwrap();
    let entry = BuiltinEntry::new(
        &["com.acme.cap.read"],
        |contribute_id, ctx, _params| match contribute_id {
            "com.acme.cap.read" => {
                // Whatever the warehouse_read backend says, surface
                // it as the success payload so the test can match on
                // the wording. `Err` paths go through
                // `DispatchError::from_kernel` instead.
                match ctx.warehouse_read().query("any_template", json!({})) {
                    Ok(rows) => Ok(json!({ "ok": rows.len() })),
                    Err(e) => Ok(json!({ "err": e.to_string() })),
                }
            }
            other => Err(Error::validation(format!("unknown: {other}"))),
        },
    );
    table.insert(ext, entry);
    Arc::new(table)
}

#[tokio::test]
async fn default_dispatcher_serves_stub_warehouse_read() {
    let (_tmp, root) = bundle_root();
    let registry = load_registry(&root);
    let table = read_calling_table();
    let dispatcher = BuiltinRestDispatcher::new(table, registry);

    let ext = ExtensionId::new("com.acme.cap").unwrap();
    let out = dispatcher
        .dispatch(&ext, "com.acme.cap.read", json!({}))
        .await
        .expect("dispatch should succeed (handler surfaces the error itself)");
    let msg = out
        .get("err")
        .and_then(|v| v.as_str())
        .expect("handler reported err key");
    assert!(
        msg.contains("not wired") && msg.contains("CapabilityFactory"),
        "expected the default stub refusal, got: {msg}"
    );
}

#[tokio::test]
async fn with_capability_factory_overrides_warehouse_read() {
    let (_tmp, root) = bundle_root();
    let registry = load_registry(&root);
    let table = read_calling_table();

    let dispatcher = BuiltinRestDispatcher::new(table, registry)
        .with_capability_factory(Arc::new(MarkerFactory));

    let ext = ExtensionId::new("com.acme.cap").unwrap();
    let out = dispatcher
        .dispatch(&ext, "com.acme.cap.read", json!({}))
        .await
        .expect("dispatch should succeed");
    let msg = out
        .get("err")
        .and_then(|v| v.as_str())
        .expect("handler reported err key");
    assert!(
        msg.contains("marker-warehouse-installed"),
        "factory override didn't reach build_ctx; got: {msg}"
    );
}

#[tokio::test]
async fn dispatcher_dispatch_unknown_extension_is_not_found() {
    // Sanity: the seam doesn't break the existing not-found path.
    let (_tmp, root) = bundle_root();
    let registry = load_registry(&root);
    let table = read_calling_table();
    let dispatcher = BuiltinRestDispatcher::new(table, registry);

    let bogus = ExtensionId::new("com.acme.does-not-exist").unwrap();
    let err = dispatcher
        .dispatch(&bogus, "com.acme.cap.read", json!({}))
        .await
        .expect_err("must report not-found");
    assert!(
        matches!(err, DispatchError::NotFound(_)),
        "got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Marker factory — distinguishable from the stub so the test can tell
// that `with_capability_factory` actually plumbed through.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MarkerFactory;

#[derive(Debug)]
struct MarkerWarehouse;
impl WarehouseReadBackend for MarkerWarehouse {
    fn query(&self, _t: &str, _p: Value) -> ExtResult<Vec<Row>> {
        Err(Error::capability("marker-warehouse-installed"))
    }
    fn count(&self, _t: &str, _p: Value) -> ExtResult<u64> {
        Err(Error::capability("marker-warehouse-installed"))
    }
    fn describe(&self, _t: &str) -> ExtResult<Option<TemplateSpec>> {
        Err(Error::capability("marker-warehouse-installed"))
    }
}

#[derive(Debug)]
struct MarkerEventBus;
impl EventBusBackend for MarkerEventBus {
    fn publish(&self, _t: &str, _p: Value) -> ExtResult<()> {
        Err(Error::capability("marker-event-bus-installed"))
    }
}

impl CapabilityFactory for MarkerFactory {
    fn warehouse_read(
        &self,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn WarehouseReadBackend> {
        Arc::new(MarkerWarehouse)
    }
    fn event_bus(
        &self,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn EventBusBackend> {
        Arc::new(MarkerEventBus)
    }
}
