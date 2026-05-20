//! StaticRegistry behaviour: append-only, look-up, stable order,
//! double-registration is loud.

use starter_authz::StaticRegistry;
use starter_spi::authz::{Ownership, ResourceRegistry, ResourceSpec};

fn spec(kind: &'static str) -> ResourceSpec {
    ResourceSpec::from_static(kind, &["read"], Ownership::None, kind, "")
}

#[test]
fn register_and_lookup() {
    let r = StaticRegistry::new();
    r.register(spec("flows"));
    r.register(spec("users"));
    assert_eq!(r.lookup("flows").map(|s| s.kind), Some("flows".into()));
    assert_eq!(r.lookup("users").map(|s| s.kind), Some("users".into()));
    assert!(r.lookup("missing").is_none());

    let known = r.known();
    let kinds: Vec<&str> = known.iter().map(|s| s.kind.as_str()).collect();
    // Sorted for stable admin-UI rendering.
    assert_eq!(kinds, vec!["flows", "users"]);
}

#[test]
#[should_panic(expected = "registered twice")]
fn duplicate_registration_panics() {
    let r = StaticRegistry::new();
    r.register(spec("flows"));
    r.register(spec("flows"));
}

#[test]
fn try_register_returns_error_on_duplicate() {
    let r = StaticRegistry::new();
    r.try_register(spec("flows")).unwrap();
    let err = r.try_register(spec("flows")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("flows"), "msg: {msg}");
}
