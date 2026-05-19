//! Convert a `starter_spi::Error` into a `(StatusCode, Json<Problem>)`
//! response. Re-exported as `IntoResponse` so handlers can `?`-bubble
//! domain errors directly.

use axum::{response::IntoResponse as AxumIntoResponse, response::Response, Json};
use starter_spi::{dto::Problem, Error};

use super::status::status_for;

/// Newtype around `starter_spi::Error` so we can implement axum's
/// `IntoResponse` for it without orphan-rule issues.
pub struct IntoResponse(pub Error);

impl AxumIntoResponse for IntoResponse {
    fn into_response(self) -> Response {
        let status = status_for(&self.0);
        let body = Problem {
            kind: kind_str(&self.0).to_string(),
            title: self.0.to_string(),
            detail: None,
        };
        (status, Json(body)).into_response()
    }
}

fn kind_str(err: &Error) -> &'static str {
    match err {
        Error::NotFound { .. } => "not_found",
        Error::Invalid { .. } => "invalid_input",
        Error::Unauthenticated => "unauthenticated",
        Error::Forbidden => "forbidden",
        Error::Conflict { .. } => "conflict",
        Error::Internal { .. } => "internal",
    }
}
