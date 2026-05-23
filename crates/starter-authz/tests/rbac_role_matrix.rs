//! Default-policy role matrix — the zero-config upgrade promise
//! (SCOPE.md R7).

use std::sync::Arc;

use starter_authz::{AuthzConfig, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec};

fn engine_with(kinds: &[&'static str]) -> StaticRbacEngine {
    let reg = Arc::new(StaticRegistry::new());
    for k in kinds {
        reg.register(ResourceSpec::from_static(
            k,
            &["read", "create", "update", "delete"],
            Ownership::Subject,
            k,
            "",
        ));
    }
    StaticRbacEngine::from_config(AuthzConfig::default(), reg).unwrap()
}

fn principal(subject: &str, role: Role) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

#[tokio::test]
async fn reader_can_read_cannot_write() {
    let eng = engine_with(&["flows"]);
    let p = principal("alice", Role::Reader);
    assert!(eng
        .check(&p, "read", &ResourceRef::collection("flows"))
        .await
        .is_allow());
    assert!(!eng
        .check(&p, "create", &ResourceRef::collection("flows"))
        .await
        .is_allow());
}

#[tokio::test]
async fn writer_can_crud_non_sensitive() {
    let eng = engine_with(&["flows"]);
    let p = principal("alice", Role::Writer);
    assert!(eng
        .check(&p, "read", &ResourceRef::collection("flows"))
        .await
        .is_allow());
    assert!(eng
        .check(&p, "create", &ResourceRef::collection("flows"))
        .await
        .is_allow());
    assert!(eng
        .check(&p, "update", &ResourceRef::collection("flows"))
        .await
        .is_allow());
}

#[tokio::test]
async fn writer_blocked_on_sensitive_create() {
    let eng = engine_with(&["users"]);
    let p = principal("alice", Role::Writer);
    // Writer can read users, but create is denied by the
    // sensitive-resources default rule.
    assert!(eng
        .check(&p, "read", &ResourceRef::collection("users"))
        .await
        .is_allow());
    let d = eng
        .check(&p, "create", &ResourceRef::collection("users"))
        .await;
    assert!(!d.is_allow(), "got {d:?}");
}

#[tokio::test]
async fn writer_can_update_own_sensitive_row() {
    let eng = engine_with(&["users"]);
    let p = principal("alice", Role::Writer);
    let own = ResourceRef::row("users", "alice").with_owner("alice");
    let other = ResourceRef::row("users", "bob").with_owner("bob");
    assert!(eng.check(&p, "update", &own).await.is_allow());
    assert!(!eng.check(&p, "update", &other).await.is_allow());
}

#[tokio::test]
async fn admin_can_do_anything() {
    let eng = engine_with(&["flows", "users", "secrets"]);
    let p = principal("admin", Role::Admin);
    for kind in ["flows", "users", "secrets"] {
        for action in ["read", "create", "update", "delete"] {
            let d = eng.check(&p, action, &ResourceRef::collection(kind)).await;
            assert!(d.is_allow(), "admin denied {action} on {kind}: {d:?}");
        }
    }
}
