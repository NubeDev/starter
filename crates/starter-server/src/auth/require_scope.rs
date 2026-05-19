//! `with_scope(router, scope)` — return 401 if no principal is
//! attached, 403 if the principal does not carry `scope`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use starter_spi::auth::{Principal, Scope};

/// Apply a scope guard to `router`.
///
/// **Layer order:** wrap this *inside* [`super::with_principal`].
/// See the `auth` module's docs for the canonical wrap pattern.
pub fn with_scope<S>(router: Router<S>, scope: Scope) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(move |req: Request<Body>, next: Next| {
        let required = scope.clone();
        async move { gate(required, req, next).await }
    }))
}

async fn gate(required: Scope, req: Request<Body>, next: Next) -> Response {
    let principal = match req.extensions().get::<Principal>() {
        Some(p) => p.clone(),
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if principal.scopes.contains(&required) {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}
