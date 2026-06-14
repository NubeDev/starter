//! Assemble the application `Router` from app state.
//!
//! Shared by `fn main` and the integration tests. `starter_server::ServerBuilder`
//! adds `/health`, `/metrics`, and `/openapi.json`; the product routes merge on
//! top. The binary additionally mounts the identity routers and wraps everything
//! in the principal layer so handlers see the authenticated `Principal`.

use std::sync::Arc;

use axum::Router;
use starter_server::auth::{csrf_guard, with_principal};
use starter_server::middleware::{accept_units_layer, PrefsResolverFor};
use starter_server::ServerBuilder;
use starter_spi::auth::Authenticator;
use starter_spi::units::{StaticRegistry, UnitRegistry};

use crate::openapi::document;
use crate::prefs::NexusPrefsResolver;
use crate::ratelimit::rate_limit_layer;
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
    tenants: Router<AppState>,
    extensions: Router<AppState>,
    setup: Router<AppState>,
    authenticator: Arc<A>,
) -> Router
where
    A: Authenticator + ?Sized,
{
    // The authz + tenant CRUD routes read `Principal` (role gate, tenant scoping,
    // the per-resource Manage check), so they need the principal layer just like
    // the product routes. The auth routes (`/auth/*`) mint the session and must
    // stay unwrapped. Each `with_principal` call adds its own layer over its
    // routes.
    // The Accept-Units layer resolves the caller's units once per request and
    // threads a `UnitsCtx` into handler extensions (see `prefs/resolver.rs`). It
    // wraps the product router *inside* the principal layer so the `Principal`
    // the resolver reads is already present when it runs.
    let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
    let resolver: Arc<dyn PrefsResolverFor> = Arc::new(NexusPrefsResolver::new(&state.prefs));
    // The rate-limit layer wraps the product router *inside* the principal layer
    // so the `Principal` it keys on is already in request extensions, and outside
    // the units layer so a throttled request does no preference resolution.
    let product = rate_limit_layer(product_router(), state.rate_limiter.clone())
        .layer(accept_units_layer(registry, resolver));
    // CSRF double-submit guard on cookie-authenticated mutations. Applied to
    // every principal-protected router (product, authz, tenants) so the whole
    // product surface enforces it uniformly — a browser session cannot be ridden
    // cross-site without echoing the `starter_csrf` cookie as `X-CSRF-Token`.
    // Bearer-token API clients and safe methods are exempt (see `csrf_guard`).
    // It wraps *inside* `with_principal` (it reads only raw cookie/header bytes,
    // so it has no principal dependency) and stays off `/auth/*`, which must mint
    // the token without already holding it.
    let protected = with_principal(csrf_guard(product), authenticator.clone());
    let authz = with_principal(csrf_guard(authz), authenticator.clone());
    let tenants = with_principal(csrf_guard(tenants), authenticator);
    ServerBuilder::<AppState>::new(state)
        .merge_router(auth)
        .merge_router(authz)
        .merge_router(tenants)
        // The extension admin router carries its own `with_principal` +
        // `with_role(Admin)` layer (applied by the kernel), so it merges as a
        // sibling here — like `authz`/`tenants` — never inside `protected`'s
        // principal layer.
        .merge_router(extensions)
        // The `/setup/*` surface carries its own `with_principal` layer (built in
        // `main` over the `RunService` state), so it merges as a sibling here —
        // like `extensions` — never inside `protected`'s principal layer.
        .merge_router(setup)
        .merge_router(protected)
        .with_openapi(document())
        .build()
}
