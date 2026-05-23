//! [`BlobError`] → HTTP status mapping.
//!
//! Centralised so every consumer of `starter-blob-axum` reports
//! the same status for the same engine-level condition. Without
//! this, two consumers wiring their own handlers would disagree on
//! e.g. whether a throttled S3 read is `429` or `503`, and the
//! API behaviour would drift per consumer.

use axum::http::StatusCode;
use starter_spi::blob::BlobError;

/// Translate a [`BlobError`] to the HTTP status the proxy returns.
///
/// Mapping is exhaustive and stable; see
/// [`crate`] docs for the table. The `Throttled.retry_after`
/// duration is *not* surfaced here — callers that need to set a
/// `Retry-After` header inspect the error directly before
/// converting.
pub fn blob_error_to_status(err: &BlobError) -> StatusCode {
    match err {
        BlobError::NotFound => StatusCode::NOT_FOUND,
        BlobError::Unauthorized => StatusCode::UNAUTHORIZED,
        BlobError::Forbidden => StatusCode::FORBIDDEN,
        BlobError::AlreadyExists => StatusCode::CONFLICT,
        BlobError::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
        BlobError::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        BlobError::Throttled { .. } => StatusCode::SERVICE_UNAVAILABLE,
        BlobError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        BlobError::Unsupported => StatusCode::NOT_IMPLEMENTED,
        BlobError::Backend(_) => StatusCode::INTERNAL_SERVER_ERROR,
        // BlobError is #[non_exhaustive]; future variants get a
        // 500 until a code review updates this mapping
        // deliberately.
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn maps_each_variant() {
        assert_eq!(blob_error_to_status(&BlobError::NotFound), 404);
        assert_eq!(blob_error_to_status(&BlobError::Unauthorized), 401);
        assert_eq!(blob_error_to_status(&BlobError::Forbidden), 403);
        assert_eq!(blob_error_to_status(&BlobError::AlreadyExists), 409);
        assert_eq!(blob_error_to_status(&BlobError::PreconditionFailed), 412);
        assert_eq!(blob_error_to_status(&BlobError::PayloadTooLarge), 413);
        assert_eq!(
            blob_error_to_status(&BlobError::Throttled {
                retry_after: Some(Duration::from_secs(2))
            }),
            503
        );
        assert_eq!(blob_error_to_status(&BlobError::Timeout), 504);
        assert_eq!(blob_error_to_status(&BlobError::Unsupported), 501);
    }
}
