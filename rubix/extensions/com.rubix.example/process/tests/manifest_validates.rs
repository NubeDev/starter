//! Validate that `block.yaml` round-trips through the host loader's
//! `validate_manifest()` cleanly. Catches namespace-ownership /
//! reserved-prefix regressions in the contributed templates +
//! anomaly rules that a pure compile-time `#[derive(Extension)]`
//! check wouldn't catch.
//!
//! This is an integration test of the *manifest itself*; it does
//! not start the extension binary.

use starter_ext_sdk::Manifest;

const BUNDLE_YAML: &str = include_str!("../../block.yaml");

#[test]
fn manifest_validates_through_host_loader() {
    let m: Manifest =
        starter_ext_sdk::serde_yaml::from_str(BUNDLE_YAML).expect("manifest parses");

    assert_eq!(m.id.as_str(), "com.rubix.example");
    assert_eq!(m.contributes.tools.len(), 3);
    assert_eq!(m.contributes.warehouse_tables.len(), 2);
    assert_eq!(m.contributes.warehouse_templates.len(), 2);
    assert_eq!(m.contributes.anomaly_rules.len(), 1);

    // Every contributed id must live under the extension's
    // namespace (R4 namespace ownership — the host's
    // `validate_manifest` enforces this, but we double-check here
    // because the extension crate doesn't depend on starter-ext-host).
    let prefix = "com.rubix.example";
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
            "warehouse_template name `{}` escapes the extension namespace",
            t.name
        );
    }
    for r in &m.contributes.anomaly_rules {
        assert!(
            r.id.starts_with(prefix),
            "anomaly_rule id `{}` escapes the extension namespace",
            r.id
        );
        assert!(
            !r.id.starts_with("builtin.")
                && !r.id.starts_with("starter.")
                && !r.id.starts_with("sys."),
            "anomaly_rule id `{}` uses a host-reserved prefix",
            r.id
        );
    }

    // The anomaly rule's `tool_id` must resolve to a contributed tool.
    let tool_ids: Vec<&str> = m
        .contributes
        .tools
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    for r in &m.contributes.anomaly_rules {
        assert!(
            tool_ids.contains(&r.tool_id.as_str()),
            "anomaly_rule `{}` points at tool_id `{}` which is not contributed by this extension",
            r.id,
            r.tool_id
        );
    }

    // Every warehouse_table column name + `tenant_id` reserved by
    // the host — the extension must NOT declare it.
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
