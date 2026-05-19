//! Server wiring: build the axum app, mount `/auth/claim` from
//! `starter-auth-token`, add a tiny consumer-owned `/hello` route
//! guarded by bearer auth.

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use prometheus::Registry;
use starter_auth_token::routes::ClaimState;
use starter_auth_token::store::SqliteClaimStore;
use starter_auth_token::TokenAuthenticator;
use starter_observability::metrics::StandardMetrics;
use starter_server::auth::with_principal;
use starter_server::ServerBuilder;
use starter_spi::auth::Principal;
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
    let protected = with_principal(protected, authenticator);

    ServerBuilder::<AppState>::new(AppState)
        .merge_router(claim_router)
        .merge_router(protected)
        .with_openapi(AppApi::openapi())
        .with_metrics(registry, metrics)
        .build()
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
