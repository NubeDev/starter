//! End-to-end tests for opt-in cache wrapping at the
//! `BuiltinRestDispatcher::dispatch` boundary.
//!
//! These exercise the integration point the v0 caching proposal
//! pins as the only call site for the v0 cut: the extension kind
//! dispatcher. They verify:
//!
//! - kinds without a sidecar take the no-op path (loader runs every
//!   call),
//! - kinds with a sidecar are single-flighted (loader runs once),
//! - `invalidate_tags` on a depended-on table tag drops the cached
//!   entry.

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use starter_cache::{CacheLayer, LayerConfig};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::builtin::{BuiltinEntry, BuiltinTable};
use starter_ext_server::{BuiltinRestDispatcher, DispatcherCache, KindCacheRegistry, RestDispatcher};
use starter_ext_spi::ExtensionId;
use tempfile::tempdir;

const BUNDLE: &str = r#"
v: 1
id: com.acme.cache
version: 0.1.0
display_name: "Cache demo"
runtime: { kind: builtin, crate_name: cache_demo }
contributes:
  tools:
    - id: com.acme.cache.usage
      input_schema:  schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/usage.md
    - id: com.acme.cache.uncached
      input_schema:  schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/uncached.md
"#;

fn write_file(root: &Path, rel: &str, body: &[u8]) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, body).unwrap();
}

fn write_bundle(root: &Path) {
    let id = "com.acme.cache";
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    write_file(&dir, "block.yaml", BUNDLE.as_bytes());
    write_file(&dir, "docs/usage.md", b"# usage");
    write_file(&dir, "docs/uncached.md", b"# uncached");
    write_file(&dir, "schemas/in.json", br#"{ "type": "object" }"#);
    write_file(&dir, "schemas/out.json", br#"{ "type": "object" }"#);
}

fn load_registry(root: &Path) -> Arc<ExtensionRegistry> {
    let recs = Loader::scan(root).validate_all();
    let mut reg = ExtensionRegistry::new();
    Loader::commit(recs, &mut reg);
    reg.seal();
    Arc::new(reg)
}

fn table_with_counter(counter: Arc<AtomicU32>) -> Arc<BuiltinTable> {
    let mut table = BuiltinTable::new();
    let ext = ExtensionId::new("com.acme.cache").unwrap();
    let entry = BuiltinEntry::new(
        &["com.acme.cache.usage", "com.acme.cache.uncached"],
        move |contribute_id, _ctx, params| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(json!({ "kind": contribute_id, "echo": params }))
        },
    );
    table.insert(ext, entry);
    Arc::new(table)
}

fn build_dispatcher_with_cache(
    counter: Arc<AtomicU32>,
    registry_entries: Vec<((ExtensionId, String), starter_cache::CacheSpec)>,
) -> (BuiltinRestDispatcher, CacheLayer, tempfile::TempDir) {
    let tmp = tempdir().unwrap();
    write_bundle(tmp.path());
    let registry = load_registry(tmp.path());
    let table = table_with_counter(counter);
    let layer = CacheLayer::new(LayerConfig::default());
    let kind_registry = KindCacheRegistry::from_entries(registry_entries);
    let dispatch =
        BuiltinRestDispatcher::new(table, registry).with_cache(DispatcherCache::new(
            layer.clone(),
            kind_registry,
        ));
    (dispatch, layer, tmp)
}

#[tokio::test]
async fn dispatch_without_sidecar_runs_every_call() {
    let counter = Arc::new(AtomicU32::new(0));
    // Registry is empty: every kind takes the no-op path.
    let (dispatch, _layer, _tmp) = build_dispatcher_with_cache(counter.clone(), vec![]);
    let ext = ExtensionId::new("com.acme.cache").unwrap();

    for _ in 0..3 {
        let _ = dispatch
            .dispatch(&ext, "com.acme.cache.uncached", json!({ "x": 1 }), None)
            .await
            .unwrap();
    }
    assert_eq!(counter.load(Ordering::SeqCst), 3, "no cache → loader runs every call");
}

#[tokio::test]
async fn dispatch_with_sidecar_caches_repeated_calls() {
    let counter = Arc::new(AtomicU32::new(0));
    let ext = ExtensionId::new("com.acme.cache").unwrap();
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant)
        .invalidate_on_table("readings");
    let (dispatch, _layer, _tmp) = build_dispatcher_with_cache(
        counter.clone(),
        vec![((ext.clone(), "com.acme.cache.usage".to_string()), spec)],
    );

    let caller = starter_ext_spi::identity::CallerIdentity {
        tenant_id: Some("tA".into()),
        user_id: Some("uX".into()),
        ..Default::default()
    };

    for _ in 0..5 {
        let v = dispatch
            .dispatch(&ext, "com.acme.cache.usage", json!({ "from": 0 }), Some(caller.clone()))
            .await
            .unwrap();
        assert_eq!(v["kind"], "com.acme.cache.usage");
    }
    assert_eq!(counter.load(Ordering::SeqCst), 1, "loader runs once across 5 hits");
}

#[tokio::test]
async fn invalidate_drops_dispatcher_cache_entry() {
    let counter = Arc::new(AtomicU32::new(0));
    let ext = ExtensionId::new("com.acme.cache").unwrap();
    let spec = starter_cache::CacheSpec::ttl(Duration::from_secs(60))
        .scope(starter_cache::CacheScope::Tenant)
        .invalidate_on_table("readings");
    let (dispatch, layer, _tmp) = build_dispatcher_with_cache(
        counter.clone(),
        vec![((ext.clone(), "com.acme.cache.usage".to_string()), spec)],
    );

    let caller = starter_ext_spi::identity::CallerIdentity {
        tenant_id: Some("tA".into()),
        user_id: Some("uX".into()),
        ..Default::default()
    };

    let _ = dispatch
        .dispatch(&ext, "com.acme.cache.usage", json!({}), Some(caller.clone()))
        .await
        .unwrap();
    let _ = dispatch
        .dispatch(&ext, "com.acme.cache.usage", json!({}), Some(caller.clone()))
        .await
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Write-path fires the table tag — next dispatch must re-run.
    layer
        .invalidator()
        .invalidate_tags(&["table:readings".to_string()])
        .await;

    let _ = dispatch
        .dispatch(&ext, "com.acme.cache.usage", json!({}), Some(caller))
        .await
        .unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2, "post-invalidate dispatch re-runs the loader");
}
