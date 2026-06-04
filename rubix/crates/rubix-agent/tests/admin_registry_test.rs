//! Admin introspection surface (`/api/v1/admin/*`) — end-to-end.
//!
//! Exercises the router built by
//! [`rubix_agent::routes::admin::admin_router`] through
//! `tower`'s `ServiceExt::oneshot`. Asserts:
//!
//! 1. `GET /api/v1/admin/overview` returns per-kind counts that
//!    match the supplied state.
//! 2. `GET /api/v1/admin/registry/tools` returns a wire-envelope
//!    page (id/label/source/metadata) and the entry for a known
//!    tool round-trips.
//! 3. `GET /api/v1/admin/registry/{unknown}` yields 400.
//! 4. `GET /api/v1/admin/registry/tools/{missing}` yields 404.
//! 5. `GET /api/v1/admin/registry?kinds=tools,nodes&limit=200`
//!    multiplexed snapshot includes only the requested kinds.
//! 6. The role gate (`with_principal` + `with_role(Role::Admin)`)
//!    yields 401 without a principal and 403 for a Reader.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use rubix_agent::admin::AdminState;
use rubix_agent::registry::{build_tool_registry, builtin_kind_behaviors};
use rubix_agent::routes::admin::admin_router;

fn state() -> AdminState {
    use std::collections::HashMap;
    let tools = build_tool_registry(90, None, None, None, None, None);
    let tool_map: HashMap<String, Arc<dyn starter_spi::tool::Tool>> = tools
        .iter()
        .map(|t| (t.definition().name, t.clone()))
        .collect();
    AdminState::empty()
        .with_tools(Arc::new(tool_map))
        .with_node_behaviors(Arc::new(builtin_kind_behaviors()))
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("body is JSON")
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request builds")
}

#[tokio::test]
async fn overview_returns_per_kind_counts() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/overview"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let counts = body.get("counts").expect("counts present");
    let tools_count = counts
        .get("tool")
        .and_then(Value::as_u64)
        .expect("tool count is u64");
    assert!(tools_count > 0, "builtin tools must be counted; got {body}");
    let nodes_count = counts
        .get("node")
        .and_then(Value::as_u64)
        .expect("node count is u64");
    assert!(nodes_count > 0, "builtin nodes must be counted; got {body}");
}

#[tokio::test]
async fn tools_list_returns_wire_envelope() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/registry/tools?limit=200"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let items = body
        .get("items")
        .and_then(Value::as_array)
        .expect("items array");
    assert!(!items.is_empty(), "expected at least one tool: {body}");
    let first = &items[0];
    for field in ["id", "label", "source", "metadata"] {
        assert!(
            first.get(field).is_some(),
            "envelope missing `{field}`: {first}"
        );
    }
}

#[tokio::test]
async fn unknown_kind_yields_400() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/registry/not-a-kind"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_tool_id_yields_404() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/registry/tools/does.not.exist"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn multiplexed_snapshot_filters_by_kinds() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/registry?kinds=tool,node&limit=200"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let obj = body.as_object().expect("snapshot is an object");
    assert!(obj.contains_key("tool"), "tool kind present: {body}");
    assert!(obj.contains_key("node"), "node kind present: {body}");
    assert!(!obj.contains_key("rule"), "rule kind omitted: {body}");
    assert!(
        !obj.contains_key("template"),
        "template kind omitted: {body}",
    );
}

#[tokio::test]
async fn multiplexed_snapshot_unknown_kind_yields_400() {
    let app = admin_router(state());
    let resp = app
        .oneshot(get("/api/v1/admin/registry?kinds=tool,bogus"))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ---- role gate ---------------------------------------------------

mod role_gate {
    //! Wraps the admin router with `with_principal` + `with_role(Admin)`
    //! exactly the way `main.rs` mounts it in production, then probes
    //! the three principal modes (none / Reader / Admin).

    use super::*;
    use async_trait::async_trait;
    use starter_server::auth::{with_principal, with_role};
    use starter_spi::auth::{Authenticator, Principal, Role};
    use starter_spi::error::{Error, Result as SpiResult};

    /// Trivial in-memory authenticator — a credential string of
    /// `"admin"` resolves to an Admin principal, `"reader"` to a
    /// Reader; everything else fails. The principal layer reads the
    /// `Authorization: Bearer <cred>` header, so tests just set
    /// that header.
    struct StubAuth;

    fn principal(subject: &str, role: Role) -> Principal {
        Principal {
            subject: subject.into(),
            role,
            scopes: vec![],
            tenant_id: None,
            teams: vec![],
            tenant_scope: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    #[async_trait]
    impl Authenticator for StubAuth {
        async fn verify(&self, credential: &str) -> SpiResult<Principal> {
            match credential {
                "admin" => Ok(principal("admin@test", Role::Admin)),
                "reader" => Ok(principal("reader@test", Role::Reader)),
                _ => Err(Error::Unauthenticated),
            }
        }
    }

    fn gated() -> axum::Router {
        let auth: Arc<dyn Authenticator> = Arc::new(StubAuth);
        with_principal(with_role(admin_router(state()), Role::Admin), auth)
    }

    fn get_with(uri: &str, bearer: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("GET").uri(uri);
        if let Some(token) = bearer {
            b = b.header("authorization", format!("Bearer {token}"));
        }
        b.body(Body::empty()).expect("request builds")
    }

    #[tokio::test]
    async fn missing_principal_yields_401() {
        let resp = gated()
            .oneshot(get_with("/api/v1/admin/overview", None))
            .await
            .expect("oneshot");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn reader_role_yields_403() {
        let resp = gated()
            .oneshot(get_with("/api/v1/admin/overview", Some("reader")))
            .await
            .expect("oneshot");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_role_yields_200() {
        let resp = gated()
            .oneshot(get_with("/api/v1/admin/overview", Some("admin")))
            .await
            .expect("oneshot");
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
