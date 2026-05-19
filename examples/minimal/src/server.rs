//! Server wiring: build the axum app, mount `/auth/claim` from
//! `starter-auth-token`, add a tiny consumer-owned `/hello` route
//! guarded by bearer auth.

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use prometheus::Registry;
use serde_json::{json, Value};
use starter_auth_token::routes::ClaimState;
use starter_auth_token::store::SqliteClaimStore;
use starter_auth_token::TokenAuthenticator;
use starter_mcp::{mcp_router, McpHttpOptions, ToolRegistry};
use starter_observability::metrics::StandardMetrics;
use starter_server::auth::with_principal;
use starter_server::ServerBuilder;
use starter_spi::auth::Principal;
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result as SpiResult;
use starter_store_sqlite::Pool;
use utoipa::OpenApi;

/// Empty app state — the example has no shared mutable state beyond
/// what the auth-token router and the principal middleware carry.
#[derive(Clone)]
pub struct AppState;

#[derive(OpenApi)]
#[openapi(paths(hello))]
struct AppApi;

/// Build the final `axum::Router`.
///
/// `SqliteClaimStore` is constructed twice over the same `Pool` (cheap
/// — `Pool` is a ref-counted handle). One instance powers the
/// `/auth/claim` route's state, the other backs the
/// `TokenAuthenticator`.
pub fn build(pool: Pool, registry: Arc<Registry>, metrics: Arc<StandardMetrics>) -> Router {
    let claim_state: ClaimState = Arc::new(SqliteClaimStore::new(pool.clone()));
    let authenticator = Arc::new(TokenAuthenticator::new(SqliteClaimStore::new(pool)));

    let claim_router = starter_auth_token::routes::claim_router::<AppState>(claim_state);

    let protected: Router<AppState> = Router::new().route("/hello", get(hello));
    let protected = with_principal(protected, authenticator.clone());

    // MCP over HTTP: same `TokenAuthenticator` enforces bearer tokens
    // on POST /mcp. Tool authors register here.
    let tools = Arc::new(ToolRegistry::new().register(EchoTool));
    let mcp = mcp_router::<AppState>(tools, McpHttpOptions::new().with_auth(authenticator));

    ServerBuilder::<AppState>::new(AppState)
        .merge_router(claim_router)
        .merge_router(protected)
        .merge_router(mcp)
        .with_openapi(AppApi::openapi())
        .with_metrics(registry, metrics)
        .build()
}

/// One example tool — echoes its arguments back. Demonstrates the
/// `Tool` trait surface without dragging in domain logic.
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "echo".into(),
            description: "Return the input arguments unchanged.".into(),
            input_schema: json!({ "type": "object", "additionalProperties": true }),
        }
    }
    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        Ok(input)
    }
}

/// `GET /hello` — returns the caller's subject. 401 if no principal
/// was attached by `with_principal`.
#[utoipa::path(
    get,
    path = "/hello",
    tag = "app",
    operation_id = "hello",
    responses(
        (status = 200, description = "Greeting for the bearer", body = String),
        (status = 401, description = "Missing or invalid bearer token"),
    ),
)]
async fn hello(principal: Option<Extension<Principal>>) -> impl IntoResponse {
    match principal {
        Some(Extension(p)) => (StatusCode::OK, format!("hello, {}\n", p.subject)),
        None => (
            StatusCode::UNAUTHORIZED,
            "missing bearer token\n".to_string(),
        ),
    }
}
