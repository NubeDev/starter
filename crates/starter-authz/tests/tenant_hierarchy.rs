//! ADR-tenant-hierarchy — subtree cross-tenant predicate semantics.
//!
//! These exercise the ENGINE side: a principal carries a resolved
//! `tenant_scope` (the subtree it administers, as the store would
//! populate at session-mint) and we assert the cross-tenant predicate
//! admits descendants, isolates siblings/parents, and still runs
//! before any rule. The store-side closure maintenance is covered by
//! the `starter-auth-users` tests.

use std::sync::Arc;

use starter_authz::{AuthzConfig, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{
    Decision, Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec,
};

fn registry_with_tenant_scoped(
    kind: &'static str,
    actions: &'static [&'static str],
) -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    r.register(ResourceSpec::from_static_tenant_scoped(
        kind,
        actions,
        Ownership::None,
        kind,
        "",
    ));
    r
}

/// A principal bound to `tenant_id` that administers `scope` (the
/// subtree, inclusive — as `TenantStore::subtree_ids` would return).
fn principal_with_scope(
    subject: &str,
    role: Role,
    tenant_id: &str,
    scope: &[&str],
) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes: vec![],
        tenant_id: Some(tenant_id.into()),
        teams: Vec::new(),
        tenant_scope: scope.iter().map(|s| (*s).to_owned()).collect(),
        extra: serde_json::Value::Null,
    }
}

fn allow_everything_cfg() -> AuthzConfig {
    AuthzConfig::from_toml_str(
        r#"
default_policy = false

[[rules]]
role     = "*"
resource = "*"
actions  = ["*"]
effect   = "allow"
"#,
    )
    .unwrap()
}

// The tree under test (matches the ADR example):
//   daikin
//   ├── acme           (daikin's client)
//   │   ├── acme-north (acme's client)
//   │   └── acme-south
//   └── byco           (another daikin client)
//
// Daikin's admin carries scope = [daikin, acme, acme-north, acme-south, byco].
// Acme's admin carries scope  = [acme, acme-north, acme-south].

#[tokio::test]
async fn parent_admin_reaches_grandchild_resource() {
    let reg = registry_with_tenant_scoped("dashboard", &["read", "write"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let daikin_admin = principal_with_scope(
        "boss",
        Role::Admin,
        "daikin",
        &["daikin", "acme", "acme-north", "acme-south", "byco"],
    );
    // A row two levels down, in acme-north.
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("acme-north");

    let d = eng.check(&daikin_admin, "write", &row).await;
    assert!(d.is_allow(), "parent admin denied on grandchild: {d:?}");
}

#[tokio::test]
async fn child_admin_reaches_its_own_child() {
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let acme_admin =
        principal_with_scope("a", Role::Admin, "acme", &["acme", "acme-north", "acme-south"]);
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("acme-south");

    let d = eng.check(&acme_admin, "read", &row).await;
    assert!(d.is_allow(), "acme admin denied on acme-south: {d:?}");
}

#[tokio::test]
async fn sibling_is_denied_cross_tenant() {
    // Acme's admin must NOT reach Byco (a sibling under daikin).
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let acme_admin =
        principal_with_scope("a", Role::Admin, "acme", &["acme", "acme-north", "acme-south"]);
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("byco");

    let d = eng.check(&acme_admin, "read", &row).await;
    match d {
        Decision::Deny { reason, .. } => assert_eq!(reason, "cross_tenant", "got {reason:?}"),
        Decision::Allow { .. } => panic!("acme reached sibling byco: {d:?}"),
    }
}

#[tokio::test]
async fn child_cannot_reach_parent_upward() {
    // Acme's admin must NOT reach a daikin-level row (upward).
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let acme_admin =
        principal_with_scope("a", Role::Admin, "acme", &["acme", "acme-north", "acme-south"]);
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("daikin");

    let d = eng.check(&acme_admin, "read", &row).await;
    match d {
        Decision::Deny { reason, .. } => assert_eq!(reason, "cross_tenant", "got {reason:?}"),
        Decision::Allow { .. } => panic!("acme reached parent daikin: {d:?}"),
    }
}

#[tokio::test]
async fn own_tenant_still_works_at_depth_zero() {
    // A leaf with scope == [self] behaves like the flat R11 case.
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let leaf = principal_with_scope("u", Role::Reader, "acme-north", &["acme-north"]);
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("acme-north");

    let d = eng.check(&leaf, "read", &row).await;
    assert!(d.is_allow(), "leaf denied on own tenant: {d:?}");
}

#[tokio::test]
async fn empty_scope_falls_back_to_strict_equality() {
    // A principal whose authenticator never populated tenant_scope
    // (pre-hierarchy wiring) must behave EXACTLY like flat R11:
    // own tenant allowed, anything else cross_tenant.
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let flat = principal_with_scope("u", Role::Reader, "acme", &[]); // empty scope

    let own = ResourceRef::row("dashboard", "d-1").with_tenant("acme");
    assert!(
        eng.check(&flat, "read", &own).await.is_allow(),
        "empty-scope principal denied on own tenant"
    );

    let other = ResourceRef::row("dashboard", "d-2").with_tenant("acme-north");
    match eng.check(&flat, "read", &other).await {
        Decision::Deny { reason, .. } => assert_eq!(reason, "cross_tenant", "got {reason:?}"),
        Decision::Allow { .. } => panic!("empty-scope principal reached a non-self tenant"),
    }
}

#[tokio::test]
async fn misconfigured_global_allow_cannot_escape_subtree() {
    // Even with the widest possible allow rule, a tenant outside the
    // subtree denies — the predicate runs before any rule.
    let reg = registry_with_tenant_scoped("dashboard", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let acme_admin =
        principal_with_scope("a", Role::Admin, "acme", &["acme", "acme-north", "acme-south"]);
    let outside = ResourceRef::row("dashboard", "d-1").with_tenant("daikin");

    match eng.check(&acme_admin, "read", &outside).await {
        Decision::Deny { reason, .. } => assert_eq!(reason, "cross_tenant", "got {reason:?}"),
        Decision::Allow { .. } => panic!("global allow leaked across subtree boundary"),
    }
}

#[tokio::test]
async fn parent_rule_applies_to_descendant_resource() {
    // A rule scoped to the parent tenant (daikin) applies when the
    // daikin admin acts on a descendant's (acme-north) resource.
    let reg = registry_with_tenant_scoped("dashboard", &["write"]);
    let cfg = AuthzConfig::from_toml_str(
        r#"
default_policy = false

[[rules]]
role      = "admin"
resource  = "dashboard"
actions   = ["write"]
tenant_id = "daikin"
effect    = "allow"
"#,
    )
    .unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let daikin_admin = principal_with_scope(
        "boss",
        Role::Admin,
        "daikin",
        &["daikin", "acme", "acme-north"],
    );
    let row = ResourceRef::row("dashboard", "d-1").with_tenant("acme-north");

    let d = eng.check(&daikin_admin, "write", &row).await;
    assert!(d.is_allow(), "daikin-scoped rule didn't reach descendant: {d:?}");
}
