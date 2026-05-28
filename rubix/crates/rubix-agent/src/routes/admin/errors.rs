//! Map a [`PageError`] onto a 400 response.
//!
//! Kept in its own file so every list handler can use the same
//! shape: `?` the projection or pagination into a `Response`
//! without sprouting a bespoke error type per route.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::admin::paging::PageError;

/// Convert a [`PageError`] into a JSON 400 body.
pub(super) fn page_error_response(err: PageError) -> Response {
    let body = json!({
        "error": "bad_request",
        "message": err.to_string(),
    });
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

/// Return a 404 with a uniform body when an id lookup misses.
pub(super) fn not_found(kind: &str, id: &str) -> Response {
    let body = json!({
        "error": "not_found",
        "kind": kind,
        "id": id,
    });
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}
