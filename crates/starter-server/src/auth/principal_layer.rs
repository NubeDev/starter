//! `with_principal(router, authenticator)` — extract a credential
//! from the request and resolve it to a `Principal` via the supplied
//! `Authenticator`, inserting the result as a request extension.
//!
//! Order of credential extraction:
//! 1. `Authorization: Bearer <value>` header — passed through
//!    verbatim to the authenticator.
//! 2. `Cookie: starter_session=<value>` — extracted from the cookie
//!    header.
//!
//! Requests without either are passed through without a `Principal`
//! extension; downstream guards turn that into 401. This keeps the
//! layer composable on routers that mix public and protected routes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;
use starter_spi::auth::Authenticator;

const SESSION_COOKIE_NAME: &str = "starter_session";

/// Apply principal extraction to `router` using `authenticator`.
pub fn with_principal<S, A>(router: Router<S>, authenticator: Arc<A>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    A: Authenticator,
{
    router.layer(from_fn(move |req: Request<Body>, next: Next| {
        let authenticator = authenticator.clone();
        async move { extract(authenticator, req, next).await }
    }))
}

async fn extract<A: Authenticator>(
    authenticator: Arc<A>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if let Some(credential) = credential_from_request(&req) {
        if let Ok(principal) = authenticator.verify(&credential).await {
            req.extensions_mut().insert(principal);
        }
    }
    next.run(req).await
}

fn credential_from_request(req: &Request<Body>) -> Option<String> {
    if let Some(v) = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(stripped) = v
            .strip_prefix("Bearer ")
            .or_else(|| v.strip_prefix("bearer "))
        {
            return Some(stripped.trim().to_string());
        }
    }
    for header in req.headers().get_all(COOKIE) {
        if let Ok(s) = header.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    if k == SESSION_COOKIE_NAME {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}
