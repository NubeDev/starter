//! Assemble the application `Router` from app state.
//!
//! Shared by `fn main` and the integration tests. `starter_server::ServerBuilder`
//! adds `/health`, `/metrics`, and `/openapi.json`; the product routes merge on
//! top. The binary additionally mounts the identity routers and wraps everything
//! in the principal layer so handlers see the authenticated `Principal`.

use std::sync::Arc;

use axum::Router;
use starter_server::auth::with_principal;
use starter_server::ServerBuilder;
use starter_spi::auth::Authenticator;

use crate::openapi::document;
use crate::routes::product_router;
use crate::state::AppState;

/// Product-only router with no identity layer. Handlers read `Principal` as an
/// `Option`, so this serves the routes that do not require authentication and is
/// what tests that inject context differently use.
pub fn router(state: AppState) -> Router {
    ServerBuilder::<AppState>::new(state)
        .merge_router(product_router())
        .with_openapi(document())
        .build()
}

/// The full server: identity routers mounted, product routes wrapped in the
/// principal layer so `Principal` is extracted from the Bearer token / session,
/// and the OpenAPI document served. `auth` and `authz` are the routers from the
/// starter crates; `authenticator` verifies credentials into a `Principal`.
pub fn assemble<A>(
    state: AppState,
    auth: Router<AppState>,
    authz: Router<AppState>,
    authenticator: Arc<A>,
) -> Router
where
    A: Authenticator + ?Sized,
{
    // The authz admin routes read `Principal` (role gate, tenant scoping, the
    // per-resource Manage check), so they need the principal layer just like the
    // product routes. The auth routes (`/auth/*`) mint the session and must stay
    // unwrapped. Each `with_principal` call adds its own layer over its routes.
    let protected = with_principal(product_router(), authenticator.clone());
    let authz = with_principal(authz, authenticator);
    ServerBuilder::<AppState>::new(state)
        .merge_router(auth)
        .merge_router(authz)
        .merge_router(protected)
        .with_openapi(document())
        .build()
}
