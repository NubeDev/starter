//! WS-18: the demo manifests parse and pass semantic validation, exercising the
//! new `contributes.provides[]`, `requires_extensions[]`, and
//! `Capability::Extension` surfaces end-to-end through the real loader path.
use starter_ext_host::validate::validate_manifest;
use starter_ext_spi::{Capability, Manifest};

fn load(rel: &str) -> Manifest {
    let path = format!(
        "{}/../../../nexus/extensions/{rel}",
        env!("CARGO_MANIFEST_DIR")
    );
    let yaml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_yaml::from_str(&yaml).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

#[test]
fn geocode_callee_manifest_is_valid() {
    let m = load("com.acme.geocode/block.yaml");
    validate_manifest(&m).expect("geocode manifest valid");
    assert!(m
        .contributes
        .provides
        .iter()
        .any(|p| p.id == "com.acme.geocode.lookup"));
}

#[test]
fn sites_caller_manifest_is_valid() {
    let m = load("com.acme.sites/block.yaml");
    validate_manifest(&m).expect("sites manifest valid");
    assert_eq!(m.requires_extensions.len(), 1);
    assert_eq!(m.requires_extensions[0].id.as_str(), "com.acme.geocode");
    assert!(m
        .capabilities
        .iter()
        .any(|c| matches!(c, Capability::Extension { .. })));
    assert!(m
        .capabilities
        .iter()
        .any(|c| matches!(c, Capability::EventBus { .. })));
}
