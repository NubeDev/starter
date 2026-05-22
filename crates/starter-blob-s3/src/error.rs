//! AWS SDK error → [`BlobError`] mapping.
//!
//! Centralised here so every operation routes through the same
//! translation. Three production cases the SCOPE calls out
//! explicitly:
//!
//! - HTTP `403` → [`BlobError::Forbidden`] (never collapsed onto
//!   `NotFound`; collapsing hides permission bugs).
//! - HTTP `404` → [`BlobError::NotFound`] (never collapsed onto
//!   `Forbidden`; that would let a non-existent key impersonate a
//!   permissions response and confuse operator dashboards).
//! - S3 `SlowDown` (`503`, `429`) → [`BlobError::Throttled`] with
//!   `retry_after` parsed from the `Retry-After` header when the
//!   server supplied one (cap at 300 s — typical S3 envelope —
//!   so a misconfigured peer cannot stall a caller indefinitely).

use std::time::Duration;

use aws_sdk_s3::error::SdkError;
use starter_spi::blob::BlobError;

/// Map a generic AWS SDK error to the trait's typed surface.
///
/// `op` is included in the `Backend` residual so a stray
/// `BlobError::Backend("...")` log line still names the operation
/// that produced it. Engines call this from every method that
/// returns a `Result<_, SdkError<E, _>>`.
pub fn map_sdk_error<E, R>(op: &'static str, err: SdkError<E, R>) -> BlobError
where
    E: std::error::Error + Send + Sync + 'static,
    R: HasStatus + std::fmt::Debug + Send + Sync + 'static,
{
    // First try the HTTP status — every SdkError carries the raw
    // response when the failure came from the wire, and that is the
    // signal we want (S3 returns the same `NoSuchKey`/`AccessDenied`
    // shapes across every distribution).
    if let Some(status) = sdk_error_status(&err) {
        match status {
            404 => return BlobError::NotFound,
            401 => return BlobError::Unauthorized,
            403 => return BlobError::Forbidden,
            409 => return BlobError::AlreadyExists,
            412 => return BlobError::PreconditionFailed,
            413 => return BlobError::PayloadTooLarge,
            429 | 503 => {
                return BlobError::Throttled {
                    retry_after: sdk_error_retry_after(&err),
                }
            }
            _ => {}
        }
    }
    if matches!(err, SdkError::TimeoutError(_)) {
        return BlobError::Timeout;
    }
    BlobError::backend(SdkErrorWrapper {
        op,
        message: err.to_string(),
    })
}

/// Borrow the HTTP status from any [`SdkError`] variant that carries
/// a raw response.
///
/// The raw response is exposed via `.raw()` on the
/// `ServiceError` / `ResponseError` variants; we only handle those
/// two because they are the only ones that carry a `Response`.
/// Constructor / dispatch / IO failures land in the `Backend`
/// residual via the caller.
fn sdk_error_status<E, R>(err: &SdkError<E, R>) -> Option<u16>
where
    R: HasStatus,
{
    use aws_sdk_s3::error::SdkError as Inner;
    match err {
        Inner::ServiceError(e) => Some(e.raw().status()),
        Inner::ResponseError(e) => Some(e.raw().status()),
        _ => None,
    }
}

/// Parse `Retry-After` from the raw response if present.
fn sdk_error_retry_after<E, R>(err: &SdkError<E, R>) -> Option<Duration>
where
    R: HasStatus,
{
    use aws_sdk_s3::error::SdkError as Inner;
    let raw: &str = match err {
        Inner::ServiceError(e) => e.raw().retry_after_header()?,
        Inner::ResponseError(e) => e.raw().retry_after_header()?,
        _ => return None,
    };
    let secs: u64 = raw.parse().ok()?;
    // Cap so a malformed peer cannot stall the caller for an hour.
    Some(Duration::from_secs(secs.min(300)))
}

/// Sealed helper trait so the status / header extraction works
/// uniformly across the SDK's raw response types without naming
/// the concrete `HttpResponse` at every call site. Implemented
/// blanket-style for any type that exposes the AWS SDK shape.
pub trait HasStatus {
    /// HTTP status code from the response.
    fn status(&self) -> u16;
    /// Borrow the `Retry-After` header value, if present.
    fn retry_after_header(&self) -> Option<&str>;
}

impl HasStatus for aws_smithy_runtime_api::client::orchestrator::HttpResponse {
    fn status(&self) -> u16 {
        self.status().as_u16()
    }
    fn retry_after_header(&self) -> Option<&str> {
        self.headers().get("retry-after")
    }
}

/// `()` impl exists so tests can synthesise a `SdkError<E, ()>` —
/// the SDK's own `timeout_error` constructor produces this shape,
/// and it has no response to inspect.
impl HasStatus for () {
    fn status(&self) -> u16 {
        0
    }
    fn retry_after_header(&self) -> Option<&str> {
        None
    }
}

/// Internal helper so the `Backend` residual carries a useful
/// `Display` without leaking the SDK's own type names through the
/// trait surface.
#[derive(Debug, thiserror::Error)]
#[error("s3 {op} failed: {message}")]
struct SdkErrorWrapper {
    op: &'static str,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_maps_to_timeout() {
        // We construct a synthetic timeout via the SDK's helper. The
        // mapper must not lose the variant.
        let err: SdkError<std::io::Error, ()> = SdkError::timeout_error(Box::new(
            std::io::Error::new(std::io::ErrorKind::TimedOut, "boom"),
        ));
        assert!(matches!(map_sdk_error("put", err), BlobError::Timeout));
    }
}
