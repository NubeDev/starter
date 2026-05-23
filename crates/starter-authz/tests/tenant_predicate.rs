//! Phase 7a — cross-tenant predicate semantics. SCOPE-EXT.md R11.

use std::sync::Arc;

use starter_authz::{AuthzConfig, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{
    Decision, Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec,
};

fn registry_with_tenant_scoped(kind: &'static str, actions: &'static [&'static str]) -> Arc<StaticRegistry> {
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

fn registry_global(kind: &'static str, actions: &'static [&'static str]) -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    r.register(ResourceSpec::from_static(
        kind,
        actions,
        Ownership::None,
        kind,
        "",
    ));
    r
}

fn principal(subject: &str, role: Role, tenant_id: Option<&str>) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes: vec![],
        tenant_id: tenant_id.map(str::to_owned),
        extra: serde_json::Value::Null,
    }
}

fn allow_everything_cfg() -> AuthzConfig {
    // The widest possible allow — role:"*" resource:"*" actions:["*"].
    // The whole point of the smoke test is that the cross-tenant deny
    // still fires WITHOUT consulting this rule.
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

#[tokio::test]
async fn cross_tenant_request_is_denied_before_any_rule_evaluates() {
    // Two tenants, one shared "allow everything" rule. A request
    // from tenant A against an object owned by tenant B must
    // deny with reason="cross_tenant" — the engine never reaches
    // the catch-all allow.
    let reg = registry_with_tenant_scoped("weather", &["read", "refresh"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let alice_in_a = principal("alice", Role::Writer, Some("tenant-a"));
    let row_in_b = ResourceRef::row("weather", "row-1").with_tenant("tenant-b");

    let d = eng.check(&alice_in_a, "read", &row_in_b).await;
    match d {
        Decision::Deny { reason, .. } => {
            assert_eq!(reason, "cross_tenant", "expected cross_tenant, got {reason:?}");
        }
        Decision::Allow { .. } => panic!("cross-tenant allowed: {d:?}"),
    }
}

#[tokio::test]
async fn principal_without_tenant_against_scoped_kind_is_no_tenant_binding() {
    // A consumer that has wired `starter-auth-token` (tenantless
    // Principal) but declared a resource as `tenant_scoped = true`
    // must see deny=no_tenant_binding — distinct from cross_tenant
    // so the operator can tell "wiring bug" from "client bug".
    let reg = registry_with_tenant_scoped("weather", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let tenantless = principal("svc", Role::Admin, None);
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    let d = eng.check(&tenantless, "read", &row).await;
    match d {
        Decision::Deny { reason, .. } => {
            assert_eq!(reason, "no_tenant_binding", "got {reason:?}");
        }
        Decision::Allow { .. } => panic!("tenantless allowed against scoped kind: {d:?}"),
    }
}

#[tokio::test]
async fn same_tenant_passes_predicate_and_falls_through_to_rule() {
    let reg = registry_with_tenant_scoped("weather", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let alice = principal("alice", Role::Reader, Some("tenant-a"));
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    let d = eng.check(&alice, "read", &row).await;
    assert!(d.is_allow(), "same-tenant denied: {d:?}");
}

#[tokio::test]
async fn global_resource_bypasses_predicate() {
    // `tenant_scoped = false` (the default) means the predicate
    // is skipped entirely — pre-Phase-7 behaviour for globally-
    // scoped kinds like `users`, `tenants`, `extensions`.
    let reg = registry_global("users", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let tenantless = principal("svc", Role::Admin, None);
    let row = ResourceRef::row("users", "u-1");
    let d = eng.check(&tenantless, "read", &row).await;
    assert!(d.is_allow(), "global resource denied for tenantless: {d:?}");
}

#[tokio::test]
async fn super_admin_sentinel_bypasses_cross_tenant_predicate() {
    let reg = registry_with_tenant_scoped("weather", &["read"]);
    let eng = StaticRbacEngine::from_config(allow_everything_cfg(), reg).unwrap();

    let super_admin = principal("ops", Role::Admin, Some("*"));
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    let d = eng.check(&super_admin, "read", &row).await;
    assert!(d.is_allow(), "super-admin sentinel denied: {d:?}");
}

#[tokio::test]
async fn tenant_scoped_rule_only_matches_its_tenant() {
    // A rule with tenant_id="tenant-a" must not match a principal
    // bound to tenant-b — even if everything else (role, resource,
    // action) lines up.
    let reg = registry_with_tenant_scoped("weather", &["refresh"]);
    let cfg = AuthzConfig::from_toml_str(
        r#"
default_policy = false

[[rules]]
role      = "writer"
resource  = "weather"
actions   = ["refresh"]
tenant_id = "tenant-a"
effect    = "allow"
"#,
    )
    .unwrap();
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let alice_a = principal("alice", Role::Writer, Some("tenant-a"));
    let bob_b = principal("bob", Role::Writer, Some("tenant-b"));
    let row_a = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    let row_b = ResourceRef::row("weather", "row-2").with_tenant("tenant-b");

    let d_alice = eng.check(&alice_a, "refresh", &row_a).await;
    assert!(d_alice.is_allow(), "tenant-a rule denied for tenant-a: {d_alice:?}");

    let d_bob = eng.check(&bob_b, "refresh", &row_b).await;
    // Cross-tenant predicate doesn't fire (both tenants align),
    // but the tenant-a rule shouldn't match tenant-b — only
    // catch-all "no_matching_rule" remains.
    match d_bob {
        Decision::Deny { reason, .. } => {
            assert_eq!(reason, "no_matching_rule", "got {reason:?}");
        }
        Decision::Allow { .. } => panic!("tenant-a rule leaked to tenant-b: {d_bob:?}"),
    }
}
