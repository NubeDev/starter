//! Phase 3 smoke tests for the DB-backed engine + admin routes.
//! SCOPE.md "Phase 3 — DB-backed engine and admin routes":
//!
//! - `dry-run-matches-real-check` — `POST /v1/authz/check` agrees
//!   with what the engine would emit on a real call.
//! - `admin-routes-require-admin` — Reader/Writer → 403.
//! - `rule-write-invalidates-cache` — inserting a deny via the
//!   REST route flips the next `check()` result.
//! - `admin-cannot-lock-themselves-out` — admin who denies
//!   themselves everything can still DELETE rules.
//! - `denial-logs-are-greppable` — engine emits the documented
//!   reason codes from SCOPE.md R9.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use starter_authz::routes::AuthzRoutesState;
use starter_authz::store::{PolicyStore, SqlitePolicyStore, StoredRule, AUTHZ_SQLITE_MIGRATOR};
use starter_authz::{authz_router, DbPolicyEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role, Scope};
use starter_spi::authz::{Decision, Ownership, PolicyEngine, ResourceRef, ResourceSpec};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};
use tower::ServiceExt;

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_authz",
            migrator: &AUTHZ_SQLITE_MIGRATOR,
        })
        .run()
        .await
        .expect("authz migrations apply");
    pool
}

fn registry() -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    r.register_spec(ResourceSpec::from_static(
        "flows",
        &["read", "create", "update", "delete"],
        Ownership::Subject,
        "Flows",
        "Test resource.",
    ));
    r
}

async fn db_engine(pool: Pool) -> (Arc<DbPolicyEngine>, Arc<dyn PolicyStore>) {
    let store: Arc<dyn PolicyStore> = Arc::new(SqlitePolicyStore::new(pool));
    let engine = DbPolicyEngine::new(store.clone(), registry(), true)
        .await
        .expect("engine builds");
    (Arc::new(engine), store)
}

/// Attach an Admin principal extension to every request — mimics
/// what the upstream `auth` middleware would have done in a real
/// server.
fn with_admin_extension(router: Router) -> Router {
    use axum::body::Body;
    use axum::http::Request;
    use axum::middleware::{from_fn, Next};
    let principal = Principal {
        subject: "admin@example.com".to_string(),
        role: Role::Admin,
        scopes: vec![Scope("admin".to_string())],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    router.layer(from_fn(move |mut req: Request<Body>, next: Next| {
        let p = principal.clone();
        async move {
            req.extensions_mut().insert(p);
            next.run(req).await
        }
    }))
}

fn with_role_extension(router: Router, role: Role) -> Router {
    use axum::middleware::{from_fn, Next};
    let principal = Principal {
        subject: "user@example.com".to_string(),
        role,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    router.layer(from_fn(move |mut req: Request<Body>, next: Next| {
        let p = principal.clone();
        async move {
            req.extensions_mut().insert(p);
            next.run(req).await
        }
    }))
}

async fn admin_app() -> (Router, Arc<DbPolicyEngine>) {
    let pool = fresh_pool().await;
    let (engine, _) = db_engine(pool).await;
    let router: Router = authz_router(AuthzRoutesState {
        engine: engine.clone(),
        registry: registry(),
        decision_sink: None,
        instances: None,
    });
    (with_admin_extension(router), engine)
}

fn csrf_headers() -> Vec<(header::HeaderName, &'static str)> {
    vec![
        (header::COOKIE, "starter_csrf=tok"),
        (header::HeaderName::from_static("x-csrf-token"), "tok"),
    ]
}

fn req_json(method: &str, uri: &str, body: Value, with_csrf: bool) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if with_csrf {
        for (k, v) in csrf_headers() {
            b = b.header(k, v);
        }
    }
    b.body(Body::from(body.to_string())).unwrap()
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn list_rules_starts_empty() {
    let (app, _) = admin_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/authz/rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["rules"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn admin_routes_require_admin() {
    let pool = fresh_pool().await;
    let (engine, _) = db_engine(pool).await;
    let router: Router = authz_router(AuthzRoutesState {
        engine,
        registry: registry(),
        decision_sink: None,
        instances: None,
    });
    let app = with_role_extension(router, Role::Writer);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/authz/rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rule_write_invalidates_cache() {
    let pool = fresh_pool().await;
    let (engine, _) = db_engine(pool).await;
    let registry = registry();
    let router: Router = authz_router(AuthzRoutesState {
        engine: engine.clone(),
        registry,
        decision_sink: None,
        instances: None,
    });
    let app = with_admin_extension(router);

    // Baseline: default policy allows Reader to read flows.
    let reader = Principal {
        subject: "r@example.com".to_string(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    let before = engine
        .check(&reader, "read", &ResourceRef::collection("flows"))
        .await;
    assert!(matches!(before, Decision::Allow { .. }));

    // Insert a high-priority deny for reader.read on flows.
    let body = json!({
        "role": "reader",
        "resource": "flows",
        "actions": ["read"],
        "effect": "deny",
        "priority": 100,
    });
    let resp = app
        .oneshot(req_json("POST", "/v1/authz/rules", body, true))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // After: deny should win.
    let after = engine
        .check(&reader, "read", &ResourceRef::collection("flows"))
        .await;
    match after {
        Decision::Deny { reason, .. } => assert_eq!(reason, "explicit_deny"),
        other => panic!("expected deny, got {other:?}"),
    }
}

#[tokio::test]
async fn dry_run_matches_real_check() {
    let pool = fresh_pool().await;
    let (engine, _) = db_engine(pool).await;
    let registry = registry();
    let router: Router = authz_router(AuthzRoutesState {
        engine: engine.clone(),
        registry,
        decision_sink: None,
        instances: None,
    });
    let app = with_admin_extension(router);

    let req_body = json!({
        "principal": { "subject": "alice@example.com", "role": "reader" },
        "action": "read",
        "resource": { "kind": "flows" },
    });
    let resp = app
        .oneshot(req_json("POST", "/v1/authz/check", req_body, false))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["decision"], "allow");

    let principal = Principal {
        subject: "alice@example.com".to_string(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    let real = engine
        .check(&principal, "read", &ResourceRef::collection("flows"))
        .await;
    assert!(matches!(real, Decision::Allow { .. }));
}

#[tokio::test]
async fn admin_cannot_lock_themselves_out() {
    let pool = fresh_pool().await;
    let (engine, store) = db_engine(pool).await;
    let registry = registry();

    // Pre-load a rule that denies admin role on rules resource —
    // simulates the worst-case "admin set the world to deny".
    // We register a synthetic `policy_rules` resource so the rule
    // is syntactically valid; what matters for this test is the
    // admin-gate route guard, not the engine itself.
    let bad = StoredRule {
        id: "bad-1".into(),
        role: "admin".into(),
        resource: "*".into(),
        actions: vec!["*".into()],
        condition: None,
        effect: "deny".into(),
        priority: 999,
        created_by: "admin".into(),
        tenant_id: None,
        source: "manual".into(),
        resource_id: None,
    };
    store.insert_rule(&bad).await.unwrap();
    engine.reload().await.unwrap();

    let router: Router = authz_router(AuthzRoutesState {
        engine: engine.clone(),
        registry,
        decision_sink: None,
        instances: None,
    });
    let app = with_admin_extension(router);

    // DELETE the offending rule. The admin-role gate (not the
    // engine) is what authorizes this, so it must still succeed.
    let resp = app
        .oneshot(req_json(
            "DELETE",
            "/v1/authz/rules/bad-1",
            Value::Null,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn denial_logs_are_greppable() {
    // The engine emits stable reason codes from the documented set
    // (SCOPE.md R9). We don't capture logs here; we just round-trip
    // a denial and assert the reason string matches the schema.
    let pool = fresh_pool().await;
    let (engine, store) = db_engine(pool).await;
    // Insert an explicit deny.
    store
        .insert_rule(&StoredRule {
            id: "rule-1".into(),
            role: "*".into(),
            resource: "flows".into(),
            actions: vec!["create".into()],
            condition: None,
            effect: "deny".into(),
            priority: 50,
            created_by: "admin".into(),
            tenant_id: None,
            source: "manual".into(),
            resource_id: None,
        })
        .await
        .unwrap();
    engine.reload().await.unwrap();

    let writer = Principal {
        subject: "w@example.com".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    let d = engine
        .check(&writer, "create", &ResourceRef::collection("flows"))
        .await;
    match d {
        Decision::Deny { reason, .. } => {
            assert!(
                [
                    "explicit_deny",
                    "not_owner",
                    "no_matching_rule",
                    "unknown_resource"
                ]
                .contains(&reason.as_str()),
                "unexpected reason: {reason}",
            );
        }
        other => panic!("expected deny, got {other:?}"),
    }
}
