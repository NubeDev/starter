//! Smoke test that the v0 canary sidecar
//! (`rubix/extensions/com.nubeio.rubixos/kinds/com.nubeio.rubixos.warehouse_query.cache.yaml`)
//! parses with the exact `CacheSpec` the proposal expects. This
//! pins the canary against silent edits — if someone changes the
//! sidecar to a v1+ shape (time_series, inner_scope, …), this test
//! catches it before the rubix host starts ignoring fields.

use std::path::PathBuf;

#[test]
fn canary_sidecar_parses() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Climb crates/starter-cache -> workspace root.
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root has two ancestors above crate dir");
    let yaml_path = workspace_root.join(
        "rubix/extensions/com.nubeio.rubixos/kinds/\
         com.nubeio.rubixos.warehouse_query.cache.yaml",
    );

    let yaml = std::fs::read_to_string(&yaml_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", yaml_path.display()));
    let spec = starter_cache::CacheSidecar::from_yaml(&yaml)
        .expect("sidecar parses")
        .into_spec()
        .expect("spec materialises");

    assert_eq!(spec.ttl, std::time::Duration::from_secs(60));
    assert_eq!(spec.scope, starter_cache::CacheScope::User);
    assert_eq!(
        spec.derived_tags(),
        vec!["table:com_nubeio_rubixos__histories".to_string()]
    );
}
