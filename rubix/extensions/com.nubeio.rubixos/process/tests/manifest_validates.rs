//! `block.yaml` round-trips through the host loader's
//! `validate_manifest()` cleanly. Catches namespace-ownership /
//! reserved-prefix regressions in the contributed templates and
//! reserved-column collisions in `warehouse_tables[]` that a pure
//! compile-time `#[derive(Extension)]` check wouldn't catch.

use starter_ext_sdk::Manifest;

const BUNDLE_YAML: &str = include_str!("../../block.yaml");

#[test]
fn manifest_validates_through_host_loader() {
    let m: Manifest =
        starter_ext_sdk::serde_yaml::from_str(BUNDLE_YAML).expect("manifest parses");

    assert_eq!(m.id.as_str(), "com.nubeio.rubixos");
    // 2 base tools (echo, warehouse_query) + 9 barcode/provisioning
    // tools (BARCODE.md §5).
    assert_eq!(m.contributes.tools.len(), 11);
    // 9 read-only dump tables (incl. the histories_1m CAGG) + 9 `bc_*`
    // provisioning catalog tables (BARCODE.md §4).
    assert_eq!(m.contributes.warehouse_tables.len(), 18);
    assert!(m.contributes.warehouse_templates.len() >= 7);

    let prefix = "com.nubeio.rubixos";
    for t in &m.contributes.tools {
        assert!(
            t.id.starts_with(prefix),
            "tool id `{}` escapes the extension namespace",
            t.id
        );
    }
    for t in &m.contributes.warehouse_templates {
        assert!(
            t.name.starts_with(prefix),
            "template name `{}` escapes the extension namespace",
            t.name
        );
    }
    for t in &m.contributes.warehouse_tables {
        for c in &t.columns {
            assert_ne!(
                c.name, "tenant_id",
                "table `{}`: extension cannot declare reserved `tenant_id` column",
                t.name
            );
        }
    }
}
