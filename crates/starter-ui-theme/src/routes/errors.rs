//! Shared helpers for turning `starter_spi::Error` and validation
//! failures into RFC 7807 responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_spi::dto::Problem;
use starter_spi::error::Error;
use starter_spi::ui::theme::TokenValueError;

pub(crate) fn problem(
    status: StatusCode,
    kind: &str,
    title: &str,
    detail: Option<String>,
) -> Response {
    let body = Problem {
        kind: kind.to_string(),
        title: title.to_string(),
        detail,
    };
    (status, Json(body)).into_response()
}

pub(crate) fn map_internal(err: Error) -> Response {
    tracing::warn!(target: "starter_ui_theme", error = %err, "theme store error");
    problem(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal",
        "internal error",
        None,
    )
}

pub(crate) fn map_token_error(err: TokenValueError) -> Response {
    problem(
        StatusCode::BAD_REQUEST,
        "invalid_input",
        "theme token failed validation",
        Some(format!(
            "token {:?} contains forbidden substring {:?}",
            err.key, err.fragment
        )),
    )
}

/// `401 Unauthorized` — caller has no `Principal` extension.
pub(crate) fn unauthorized() -> Response {
    problem(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "authentication required",
        None,
    )
}

/// `403 Forbidden` — caller is authenticated but not an admin.
pub(crate) fn forbidden() -> Response {
    problem(
        StatusCode::FORBIDDEN,
        "forbidden",
        "admin role required",
        None,
    )
}
