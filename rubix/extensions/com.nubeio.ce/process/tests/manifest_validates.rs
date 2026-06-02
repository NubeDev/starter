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

    assert_eq!(m.id.as_str(), "com.nubeio.ce");
    // echo + warehouse_query + 3 device CRUD + 3 engine REST proxy.
    assert_eq!(m.contributes.tools.len(), 8);
    // Single catalog table.
    assert_eq!(m.contributes.warehouse_tables.len(), 1);
    assert_eq!(m.contributes.warehouse_templates.len(), 2);

    let prefix = "com.nubeio.ce";
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
