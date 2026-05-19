//! `with_role(router, role)` — return 401 if no principal extension
//! is attached, 403 if the principal's role is below `role`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use starter_spi::auth::{Principal, Role};

/// Apply a role guard to `router`.
///
/// **Layer order:** wrap this *inside* [`super::with_principal`] so
/// the principal extension is present before the gate reads it. See
/// the `auth` module's docs for the canonical wrap pattern.
pub fn with_role<S>(router: Router<S>, role: Role) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(move |req: Request<Body>, next: Next| {
        let required = role;
        async move { gate(required, req, next).await }
    }))
}

async fn gate(required: Role, req: Request<Body>, next: Next) -> Response {
    let principal = match req.extensions().get::<Principal>() {
        Some(p) => p.clone(),
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if role_rank(principal.role) >= role_rank(required) {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

fn role_rank(r: Role) -> u8 {
    match r {
        Role::Reader => 0,
        Role::Writer => 1,
        Role::Admin => 2,
    }
}
