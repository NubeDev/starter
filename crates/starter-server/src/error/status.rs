//! Single source of truth for `Error → StatusCode`. If a new
//! `Error` variant lands, add it here and the rest of the surface
//! picks it up automatically.

use http::StatusCode;
use starter_spi::Error;

/// HTTP status code for a domain error.
pub fn status_for(err: &Error) -> StatusCode {
    match err {
        Error::NotFound { .. } => StatusCode::NOT_FOUND,
        Error::Invalid { .. } => StatusCode::BAD_REQUEST,
        Error::Unauthenticated => StatusCode::UNAUTHORIZED,
        Error::Forbidden => StatusCode::FORBIDDEN,
        Error::Conflict { .. } => StatusCode::CONFLICT,
        Error::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        // `Error` is `#[non_exhaustive]`; unknown variants surface as
        // 500 until a dedicated mapping is added here.
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
