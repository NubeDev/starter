//! Admin streaming invoke endpoint —
//! `POST /api/v1/admin/registry/tools/{id}/invoke/stream`.
//!
//! Mirrors [`admin_invoke_test`] but reads the SSE body and
//! asserts:
//!
//! 1. Successful invoke returns 200 and emits in order:
//!    `connected` → `result` → `done { status: "ok" }`.
//! 2. Missing / blank tenant yields 400 (no stream opened).
//! 3. Unknown tool id yields 404.
//! 4. Admin without `admin:invoke` scope yields 403.
//! 5. Missing principal yields 401.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use rubix_agent::admin::AdminState;
use rubix_agent::registry::build_tool_registry;
use rubix_agent::routes::admin::admin_invoke_stream_registrar;
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
        with_role(
            admin_invoke_stream_registrar(state()).into_router(),
            Role::Admin,
        ),
        Scope::new("admin:invoke"),
    );
    with_principal(scoped, auth)
}

fn post_stream(tool_id: &str, body: Value, bearer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/admin/registry/tools/{tool_id}/invoke/stream"
        ))
        .header("content-type", "application/json")
        .header("accept", "text/event-stream");
    if let Some(token) = bearer {
        b = b.header("authorization", format!("Bearer {token}"));
    }
    b.body(Body::from(body.to_string()))
        .expect("request builds")
}

/// Pull every `data: { ... }` line out of an SSE body and parse
/// each as a [`StreamFrame`]-shaped JSON value.
async fn collect_frames(resp: axum::response::Response) -> Vec<Value> {
    let bytes = to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("body bytes");
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8 body");
    text.lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|payload| serde_json::from_str::<Value>(payload.trim()).expect("frame json"))
        .collect()
}

#[tokio::test]
async fn successful_invoke_streams_connected_result_done() {
    let resp = gated()
        .oneshot(post_stream(
            "rubix.system.disk",
            json!({"tenant": "tenant-1", "input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let frames = collect_frames(resp).await;
    assert!(
        frames.len() >= 3,
        "expected at least 3 frames; got {frames:?}"
    );
    assert_eq!(frames[0]["type"], "connected");
    assert_eq!(frames[1]["type"], "result");
    let done = frames.last().expect("done frame");
    assert_eq!(done["type"], "done");
    assert_eq!(done["status"], "ok");
    assert!(
        done.get("latency_ms").is_some(),
        "done frame missing latency_ms: {done}"
    );
    assert!(
        done.get("input_tokens").is_none(),
        "admin done must not carry chat keys: {done}"
    );
}

#[tokio::test]
async fn missing_tenant_yields_400() {
    let resp = gated()
        .oneshot(post_stream(
            "rubix.system.disk",
            json!({"input": {}}),
            Some("admin-invoke"),
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unknown_tool_yields_404() {
    let resp = gated()
        .oneshot(post_stream(
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
        .oneshot(post_stream(
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
        .oneshot(post_stream(
            "rubix.system.disk",
            json!({"tenant": "tenant-1", "input": {}}),
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
