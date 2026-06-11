//! `csrf_guard(router)` — enforce the double-submit CSRF token on
//! cookie-authenticated mutating requests.
//!
//! The session is a cookie (`starter_session`, httpOnly) paired with a
//! non-httpOnly CSRF cookie (`starter_csrf`). A browser attached to a
//! victim's session will *send the cookies* on a cross-site forged
//! request, but JavaScript on the attacker's origin cannot read the
//! `starter_csrf` value (same-origin policy) to echo it back as the
//! `X-CSRF-Token` header. So requiring `header == cookie` proves the
//! request came from first-party code that could read the cookie —
//! the standard double-submit defence (the auth crate's `/auth/logout`
//! checks the same pair by hand; this layer generalises it to every
//! product mutation).
//!
//! **Scope — what is and isn't guarded (by design):**
//! - Only **mutating** methods (`POST`/`PUT`/`PATCH`/`DELETE`). Safe
//!   methods (`GET`/`HEAD`/`OPTIONS`) are never CSRF vectors.
//! - Only **cookie-authenticated** requests. A `Authorization: Bearer`
//!   API client carries no ambient cookie an attacker could ride, so it
//!   is exempt — requiring a CSRF header there would break every
//!   non-browser client for no security gain. "Cookie-authenticated"
//!   means: a `starter_session` cookie is present AND no `Bearer`
//!   header. (Bearer wins credential resolution in `with_principal`, so
//!   it must win here too, or a client sending both would be forced to
//!   supply a CSRF token it has no reason to hold.)
//! - Requests with **no session cookie** pass through untouched — an
//!   unauthenticated request has nothing to forge; downstream guards
//!   turn a missing principal into 401.
//!
//! Layer order: apply *inside* [`super::with_principal`] (so it runs
//! after the principal is resolved) but it reads only the raw cookie /
//! header bytes, so it does not depend on the principal extension. A
//! failing check short-circuits with `403` before the handler runs.

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::{Method, Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;

const SESSION_COOKIE_NAME: &str = "starter_session";
const CSRF_COOKIE_NAME: &str = "starter_csrf";
const CSRF_HEADER_NAME: &str = "x-csrf-token";

/// Wrap `router` so cookie-authenticated mutating requests must echo
/// the `starter_csrf` cookie back as the `X-CSRF-Token` header.
pub fn csrf_guard<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(|req: Request<Body>, next: Next| async move {
        guard(req, next).await
    }))
}

async fn guard(req: Request<Body>, next: Next) -> Response {
    if requires_csrf(&req) && !csrf_ok(&req) {
        return (
            StatusCode::FORBIDDEN,
            "CSRF token missing or mismatched",
        )
            .into_response();
    }
    next.run(req).await
}

/// A request needs the CSRF check iff it is a mutating method AND it is
/// authenticated by the session cookie rather than a bearer token.
fn requires_csrf(req: &Request<Body>) -> bool {
    if !is_mutating(req.method()) {
        return false;
    }
    // Bearer-token clients are exempt (no ambient cookie to ride).
    if has_bearer(req) {
        return false;
    }
    // Only guard requests that actually carry the session cookie.
    cookie(req, SESSION_COOKIE_NAME).is_some()
}

fn is_mutating(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn has_bearer(req: &Request<Body>) -> bool {
    req.headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
                .is_some()
        })
        .unwrap_or(false)
}

/// Double-submit: the `starter_csrf` cookie must be present and equal to
/// the `X-CSRF-Token` header. A missing cookie, missing header, or
/// mismatch all fail closed.
fn csrf_ok(req: &Request<Body>) -> bool {
    let cookie_csrf = cookie(req, CSRF_COOKIE_NAME);
    let header_csrf = req
        .headers()
        .get(CSRF_HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    matches!((cookie_csrf, header_csrf), (Some(c), Some(h)) if c == h)
}

/// Read a single cookie value from the `Cookie` header(s).
fn cookie(req: &Request<Body>, name: &str) -> Option<String> {
    for header in req.headers().get_all(COOKIE) {
        if let Ok(s) = header.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    if k.trim() == name {
                        return Some(v.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::{get, post};
    use tower::ServiceExt; // oneshot

    fn app() -> Router {
        csrf_guard(
            Router::new()
                .route("/m", post(|| async { "ok" }))
                .route("/s", get(|| async { "ok" })),
        )
    }

    fn req(method: Method, path: &str) -> axum::http::request::Builder {
        Request::builder().method(method).uri(path)
    }

    async fn status(req: Request<Body>) -> StatusCode {
        app().oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn safe_method_passes_without_token() {
        let r = req(Method::GET, "/s")
            .header(COOKIE, "starter_session=s; starter_csrf=abc")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status(r).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn cookie_mutation_without_header_is_forbidden() {
        let r = req(Method::POST, "/m")
            .header(COOKIE, "starter_session=s; starter_csrf=abc")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status(r).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn cookie_mutation_with_matching_header_passes() {
        let r = req(Method::POST, "/m")
            .header(COOKIE, "starter_session=s; starter_csrf=abc")
            .header(CSRF_HEADER_NAME, "abc")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status(r).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn cookie_mutation_with_mismatched_header_is_forbidden() {
        let r = req(Method::POST, "/m")
            .header(COOKIE, "starter_session=s; starter_csrf=abc")
            .header(CSRF_HEADER_NAME, "WRONG")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status(r).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn bearer_mutation_is_exempt() {
        // A bearer client carries no ambient cookie to forge; even if it
        // also sends a session cookie, the bearer header wins and CSRF is
        // skipped (parity with `with_principal`'s credential precedence).
        let r = req(Method::POST, "/m")
            .header(AUTHORIZATION, "Bearer tok")
            .header(COOKIE, "starter_session=s; starter_csrf=abc")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status(r).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn unauthenticated_mutation_passes_layer() {
        // No session cookie → nothing to forge; the layer is a no-op and a
        // downstream guard handles the missing principal (401), not 403.
        let r = req(Method::POST, "/m").body(Body::empty()).unwrap();
        assert_eq!(status(r).await, StatusCode::OK);
    }
}
