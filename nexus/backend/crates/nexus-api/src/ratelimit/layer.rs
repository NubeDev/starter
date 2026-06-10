//! Axum middleware that applies the per-tenant token-bucket rate limit.
//!
//! LAYER: transport (REST). It reads the authenticated `Principal` from request
//! extensions, asks the limiter for a token, and either forwards the request or
//! short-circuits with 429 + `Retry-After`. It carries no business logic — the
//! bucket arithmetic lives in `bucket.rs`. Mount it *inside* `with_principal` so
//! the `Principal` it reads is already present.
//!
//! Requests with no authenticated tenant (the principal is absent or has no
//! tenant binding) pass through untouched: rate limiting is a per-tenant fairness
//! control, and the auth/tenant layers already reject an unbound caller at the
//! handler. The dev single-datasource path is likewise unmetered.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use starter_spi::auth::Principal;

use super::TenantRateLimiter;

/// Wrap `router` so every request is checked against the calling tenant's rate
/// bucket. A throttled request never reaches the handler.
pub fn rate_limit_layer<S>(router: Router<S>, limiter: TenantRateLimiter) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(move |req: Request<Body>, next: Next| {
        let limiter = limiter.clone();
        async move { enforce(limiter, req, next).await }
    }))
}

/// Look up the tenant, spend a token, and forward — or reject with 429. Kept
/// under the transport ceiling: extract the tenant, call one domain check, map
/// the outcome to a response.
async fn enforce(limiter: TenantRateLimiter, req: Request<Body>, next: Next) -> Response {
    let tenant = req
        .extensions()
        .get::<Principal>()
        .and_then(|p| p.tenant_id.clone())
        .filter(|t| !t.is_empty());

    let Some(tenant) = tenant else {
        return next.run(req).await;
    };

    match limiter.check(&tenant).await {
        Ok(()) => next.run(req).await,
        Err(retry_after) => too_many_requests(retry_after.as_secs().max(1)),
    }
}

/// Build the 429 response with a `Retry-After` hint (whole seconds, at least 1).
fn too_many_requests(retry_after_secs: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after_secs.to_string())],
        "rate limit exceeded",
    )
        .into_response()
}
