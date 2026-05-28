//! Admin invoke endpoint — `POST /api/v1/admin/registry/tools/{id}/invoke`.
//!
//! End-to-end exercise of [`rubix_agent::routes::admin::admin_invoke_router`]
//! and its scope gate. Asserts:
//!
//! 1. A well-formed body against a known parameterless tool returns 200.
//! 2. Missing `tenant` (absent / blank / whitespace) yields 400.
//! 3. An unknown tool id yields 404.
//! 4. A principal lacking `admin:invoke` scope yields 403 even when
//!    `Role::Admin` is satisfied — the scope split is real.

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use rubix_agent::admin::AdminState;
use rubix_agent::registry::build_tool_registry;
use rubix_agent::routes::admin::admin_invoke_router;
use starter_server::auth::{with_principal, with_role, with_scope};
use starter_spi::auth::{Authenticator, Principal, Role, Scope};
use starter_spi::error::{Error, Result as SpiResult};

struct StubAuth;

fn principal(subject: &str, role: Role, scopes: Vec<Scope>) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes,
        tenant_id: None,
        teams: vec![],
        extra: Value::Null,
    }
}

#[async_trait]
impl Authenticator for StubAuth {
    async fn verify(&self, credential: &str) -> SpiResult<Principal> {
        match credential {
            "admin-invoke" => Ok(principal(
                "admin@test",
                Role::Admin,
                vec![Scope::new("admin:invoke")],
            )),
            "admin-readonly" => Ok(principal("admin@test", Role::Admin, vec![])),
            _ => Err(Error::Unauthenticated),
        }
    }
}

fn state() -> AdminState {
    use std::collections::HashMap;
    let tools = build_tool_registry(90, None, None, None, None, None);
    let tool_map: HashMap<String, Arc<dyn starter_spi::tool::Tool>> = tools
        .iter()
        .map(|t| (t.definition().name, t.clone()))
        .collect();
    AdminState::empty().with_tools(Arc::new(tool_map))
}

fn gated() -> axum::Router {
    let auth: Arc<dyn Authenticator> = Arc::new(StubAuth);
    let scoped = with_scope(
        with_role(admin_invoke_router(state()), Role::Admin),
        Scope::new("admin:invoke"),
    );
    with_principal(scoped, auth)
}

fn post_invoke(tool_id: &str, body: Value, bearer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/admin/registry/tools/{tool_id}/invoke"
        ))
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        b = b.header("authorization", format!("Bearer {token}"));
    }
    b.body(Body::from(body.to_string()))
        .expect("request builds")
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("body is JSON")
}

#[tokio::test]
async fn well_formed_invoke_returns_200() {
    // `rubix.system.disk` ships in the builtin registry and accepts
    // an empty input. It is the canonical "parameterless tool" the
    // admin surface lets an operator probe.
    let resp = gated()
        .oneshot(post_invoke(
            "rubix.system.disk",
            json!({"tenant": "tenant-1", "input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(
        body.get("summary").is_some() || body.get("rendered_summary").is_some() || body.is_object(),
        "expected a JSON object payload; got {body}",
    );
}

#[tokio::test]
async fn missing_tenant_yields_400() {
    let resp = gated()
        .oneshot(post_invoke(
            "rubix.system.disk",
            json!({"input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    // `tenant` is missing entirely → serde rejects → axum maps to 400.
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn blank_tenant_yields_400() {
    let resp = gated()
        .oneshot(post_invoke(
            "rubix.system.disk",
            json!({"tenant": "   ", "input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unknown_tool_yields_404() {
    let resp = gated()
        .oneshot(post_invoke(
            "does.not.exist",
            json!({"tenant": "tenant-1", "input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn readonly_admin_without_scope_yields_403() {
    let resp = gated()
        .oneshot(post_invoke(
            "rubix.system.disk",
            json!({"tenant": "tenant-1", "input": {}}),
            Some("admin-readonly"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_principal_yields_401() {
    let resp = gated()
        .oneshot(post_invoke(
            "rubix.system.disk",
            json!({"tenant": "tenant-1", "input": {}}),
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
