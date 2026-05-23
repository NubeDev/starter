//! Ownership condition, deny-overrides, unknown-resource default,
//! OAuth attribute rules, and `*` glob assignments. The TOML
//! shape exercised here is the one shown in SCOPE.md R6.

use std::sync::Arc;

use starter_authz::{AuthzConfig, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{
    Decision, Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec,
};

fn registry(kinds: &[(&'static str, &'static [&'static str], Ownership)]) -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    for (k, actions, own) in kinds {
        r.register(ResourceSpec::from_static(k, actions, *own, k, ""));
    }
    r
}

fn principal(subject: &str, role: Role, extra: serde_json::Value) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes: vec![],
        extra,
        tenant_id: None,
        teams: Vec::new(),
    }
}

#[tokio::test]
async fn unknown_resource_is_default_deny() {
    let reg = registry(&[]);
    let cfg = AuthzConfig::default();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();
    let p = principal("admin", Role::Admin, serde_json::Value::Null);
    let d = eng
        .check(&p, "read", &ResourceRef::collection("totally-new"))
        .await;
    match d {
        Decision::Deny { reason, .. } => assert_eq!(reason, "unknown_resource"),
        Decision::Allow { .. } => panic!("expected deny"),
    }
}

#[tokio::test]
async fn ownership_keyword_allows_owner_only() {
    let reg = registry(&[("flows", &["read", "update"], Ownership::Subject)]);

    // No default policy — we want to exercise the rule shape alone.
    let toml = r#"
        default_policy = false

        [[rules]]
        id = "writer-update-own"
        role = "writer"
        resource = "flows"
        actions = ["update"]
        condition = "owner"
        effect = "allow"
    "#;
    let cfg = AuthzConfig::from_toml_str(toml).unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let alice = principal("alice", Role::Writer, serde_json::Value::Null);
    let own = ResourceRef::row("flows", "f1").with_owner("alice");
    let bobs = ResourceRef::row("flows", "f1").with_owner("bob");

    assert!(eng.check(&alice, "update", &own).await.is_allow());

    let d = eng.check(&alice, "update", &bobs).await;
    match d {
        Decision::Deny { reason, .. } => assert_eq!(reason, "no_matching_rule"),
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn deny_overrides_allow() {
    let reg = registry(&[("flows", &["update"], Ownership::None)]);

    let toml = r#"
        default_policy = false

        [[rules]]
        id = "writers-can-update"
        role = "writer"
        resource = "flows"
        actions = ["update"]
        effect = "allow"

        [[rules]]
        id = "contractors-cannot"
        role = "*"
        resource = "flows"
        actions = ["update"]
        condition = 'oauth.email_domain == "contractor.com"'
        effect = "deny"
    "#;
    let cfg = AuthzConfig::from_toml_str(toml).unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let employee = principal(
        "alice",
        Role::Writer,
        serde_json::json!({"oauth": {"email_domain": "acme.com"}}),
    );
    let contractor = principal(
        "carol",
        Role::Writer,
        serde_json::json!({"oauth": {"email_domain": "contractor.com"}}),
    );

    assert!(eng
        .check(&employee, "update", &ResourceRef::collection("flows"))
        .await
        .is_allow());

    let d = eng
        .check(&contractor, "update", &ResourceRef::collection("flows"))
        .await;
    match d {
        Decision::Deny { reason, .. } => assert_eq!(reason, "explicit_deny"),
        Decision::Allow { .. } => panic!("contractor allowed despite deny rule"),
    }
}

#[tokio::test]
async fn oauth_attribute_rule() {
    let reg = registry(&[("deployments", &["create"], Ownership::None)]);
    let toml = r#"
        default_policy = false

        [[rules]]
        id = "acme-verified-can-deploy"
        role = "*"
        resource = "deployments"
        actions = ["create"]
        condition = 'oauth.email_domain == "acme.com" and oauth.email_verified'
        effect = "allow"
    "#;
    let cfg = AuthzConfig::from_toml_str(toml).unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let alice = principal(
        "alice",
        Role::Writer,
        serde_json::json!({"oauth": {"email_domain": "acme.com", "email_verified": true}}),
    );
    let bob = principal(
        "bob",
        Role::Writer,
        serde_json::json!({"oauth": {"email_domain": "acme.com", "email_verified": false}}),
    );
    let carol = principal("carol", Role::Writer, serde_json::Value::Null);

    assert!(eng
        .check(&alice, "create", &ResourceRef::collection("deployments"))
        .await
        .is_allow());

    for absent in [&bob, &carol] {
        let d = eng
            .check(absent, "create", &ResourceRef::collection("deployments"))
            .await;
        assert!(!d.is_allow(), "missing attr should not allow: {d:?}");
    }
}

#[tokio::test]
async fn glob_subject_assignment_grants_role() {
    let reg = registry(&[("flows", &["update"], Ownership::None)]);
    let toml = r#"
        default_policy = false

        [[assignments]]
        subject = "*@acme.com"
        roles = ["editor"]

        [[rules]]
        id = "editors-update-flows"
        role = "editor"
        resource = "flows"
        actions = ["update"]
        effect = "allow"
    "#;
    let cfg = AuthzConfig::from_toml_str(toml).unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let acme = principal("alice@acme.com", Role::Reader, serde_json::Value::Null);
    let other = principal("eve@other.com", Role::Reader, serde_json::Value::Null);

    assert!(eng
        .check(&acme, "update", &ResourceRef::collection("flows"))
        .await
        .is_allow());
    assert!(!eng
        .check(&other, "update", &ResourceRef::collection("flows"))
        .await
        .is_allow());
}
