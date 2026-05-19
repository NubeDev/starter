//! HTTP transport for the MCP dispatcher.
//!
//! Exposes a single `POST /mcp` route accepting a JSON-RPC envelope
//! and returning the response (or `204 No Content` for notifications).
//! Use [`mcp_router`] to mount alongside the rest of the consumer's
//! `axum::Router`.
//!
//! # Long-term shape
//!
//! Real MCP "Streamable HTTP" is request-then-SSE — server can push
//! progress events on the same response. v0.1 ships the request /
//! response half; the SSE upgrade is a v0.2 feature and will surface
//! as a `mcp_sse_router` sibling. Tool authors writing today should
//! keep `Tool::invoke` returning a single `Value` — when SSE lands,
//! a separate `StreamingTool` trait will opt in to chunked output
//! without breaking the existing surface.
//!
//! # Authentication seam
//!
//! Pass [`McpHttpOptions::with_auth`] to require a bearer token; the
//! handler runs the supplied `Authenticator` and surfaces a 401 on
//! failure, or attaches the resolved `Principal` as a request
//! extension so tool impls can read it via downstream middleware.
//! When no authenticator is set, the route is open (single-user or
//! network-isolated case — e.g. a sidecar serving only `localhost`).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header::AUTHORIZATION, HeaderMap, Request, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use starter_spi::auth::{Authenticator, Principal};

use crate::registry::ToolRegistry;
use crate::server::dispatch::dispatch;

/// Build options for [`mcp_router`].
///
/// Default: open route, no auth check. Call [`Self::with_auth`] to
/// require `Authorization: Bearer …`.
#[derive(Default)]
pub struct McpHttpOptions {
    authenticator: Option<Arc<dyn Authenticator>>,
}

impl McpHttpOptions {
    /// Build an empty options struct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Require a bearer credential. The handler enforces presence,
    /// runs `authenticator.verify(token)`, returns 401 on failure,
    /// and inserts the resolved `Principal` as a request extension.
    pub fn with_auth(mut self, authenticator: Arc<dyn Authenticator>) -> Self {
        self.authenticator = Some(authenticator);
        self
    }
}

/// Build the MCP HTTP router. Mount onto a starter-server `Router<S>`
/// via `ServerBuilder::merge_router`.
///
/// Generic over the consumer's `AppState` so the route merges into
/// any consumer router without state-type gymnastics.
pub fn mcp_router<S>(registry: Arc<ToolRegistry>, opts: McpHttpOptions) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let mut router: Router<S> = Router::new()
        .route("/mcp", post(handle))
        .with_state(registry);

    if let Some(authenticator) = opts.authenticator {
        router = router.layer(from_fn_with_state(authenticator, auth_layer));
    }

    router
}

async fn handle(State(registry): State<Arc<ToolRegistry>>, body: String) -> Response {
    match dispatch(&registry, &body).await {
        Some(resp) => (StatusCode::OK, Json(resp)).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

/// Middleware: extract `Authorization: Bearer <token>`, verify via the
/// supplied `Authenticator`, insert the resolved `Principal` as a
/// request extension. Returns 401 on missing/invalid credential.
async fn auth_layer(
    State(authenticator): State<Arc<dyn Authenticator>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let token = match bearer_from_headers(req.headers()) {
        Some(t) => t,
        None => return unauthorized("missing bearer token"),
    };
    match authenticator.verify(&token).await {
        Ok(principal) => {
            req.extensions_mut().insert::<Principal>(principal);
            next.run(req).await
        }
        Err(_) => unauthorized("invalid bearer token"),
    }
}

fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(AUTHORIZATION)?.to_str().ok()?;
    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
}

fn unauthorized(msg: &'static str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": Value::Null,
            "error": { "code": -32001, "message": msg },
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use starter_spi::auth::{Principal, Role};
    use starter_spi::tool::{Tool, ToolDefinition};
    use starter_spi::{Error, Result as SpiResult};

    struct EchoTool;
    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "echo".into(),
                description: "echo".into(),
                input_schema: json!({ "type": "object" }),
            }
        }
        async fn invoke(&self, input: Value) -> SpiResult<Value> {
            Ok(input)
        }
    }

    struct AcceptAll;
    #[async_trait]
    impl Authenticator for AcceptAll {
        async fn verify(&self, credential: &str) -> SpiResult<Principal> {
            Ok(Principal {
                subject: format!("token:{credential}"),
                role: Role::Admin,
                scopes: vec![],
                extra: Value::Null,
            })
        }
    }

    struct RejectAll;
    #[async_trait]
    impl Authenticator for RejectAll {
        async fn verify(&self, _credential: &str) -> SpiResult<Principal> {
            Err(Error::Unauthenticated)
        }
    }

    fn registry() -> Arc<ToolRegistry> {
        Arc::new(ToolRegistry::new().register(EchoTool))
    }

    async fn spawn_app(router: Router<()>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn open_route_dispatches_initialize() {
        let app = mcp_router::<()>(registry(), McpHttpOptions::new());
        let (base, _h) = spawn_app(app).await;

        let resp: serde_json::Value = reqwest::Client::new()
            .post(format!("{base}/mcp"))
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp["result"]["serverInfo"]["name"], "starter-mcp");
    }

    #[tokio::test]
    async fn notification_returns_204() {
        let app = mcp_router::<()>(registry(), McpHttpOptions::new());
        let (base, _h) = spawn_app(app).await;

        let resp = reqwest::Client::new()
            .post(format!("{base}/mcp"))
            .body(r#"{"jsonrpc":"2.0","method":"tools/list"}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    #[tokio::test]
    async fn auth_rejects_missing_bearer() {
        let app = mcp_router::<()>(
            registry(),
            McpHttpOptions::new().with_auth(Arc::new(AcceptAll)),
        );
        let (base, _h) = spawn_app(app).await;

        let resp = reqwest::Client::new()
            .post(format!("{base}/mcp"))
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);
    }

    #[tokio::test]
    async fn auth_accepts_valid_bearer() {
        let app = mcp_router::<()>(
            registry(),
            McpHttpOptions::new().with_auth(Arc::new(AcceptAll)),
        );
        let (base, _h) = spawn_app(app).await;

        let resp: serde_json::Value = reqwest::Client::new()
            .post(format!("{base}/mcp"))
            .bearer_auth("good-token")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(resp["id"], 1);
    }

    #[tokio::test]
    async fn auth_rejects_invalid_bearer() {
        let app = mcp_router::<()>(
            registry(),
            McpHttpOptions::new().with_auth(Arc::new(RejectAll)),
        );
        let (base, _h) = spawn_app(app).await;

        let resp = reqwest::Client::new()
            .post(format!("{base}/mcp"))
            .bearer_auth("bad")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);
    }
}
