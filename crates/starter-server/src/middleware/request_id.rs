//! Generates (or adopts) an `X-Request-Id` per incoming request and
//! attaches it as a request extension. Outbound responses echo the
//! same id back so a client correlating logs across services has a
//! stable handle.
//!
//! Exposed as a router-extension helper rather than a freestanding
//! `Layer` — axum's `from_fn` produces a closure-typed layer whose
//! exact shape is hard to spell without unstable TAIT. The wrap
//! function keeps the call site clean (`router = with_request_id(
//! router)`).

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;
use http::HeaderValue;
use starter_observability::middleware::{RequestId, REQUEST_ID_HEADER};

/// Apply the request-id middleware to `router`.
///
/// Inserts a [`RequestId`] extension that handlers can read with
/// `axum::Extension<RequestId>`; echoes the value back as the
/// `X-Request-Id` response header.
pub fn with_request_id(router: Router) -> Router {
    router.layer(from_fn(set_request_id))
}

async fn set_request_id(mut req: Request<Body>, next: Next) -> Response {
    let id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(RequestId)
        .unwrap_or_else(RequestId::generate);
    req.extensions_mut().insert(id);
    let mut resp = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&id.to_string()) {
        resp.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    resp
}
