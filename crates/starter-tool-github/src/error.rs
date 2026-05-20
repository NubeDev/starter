//! [`GitHubError`] — the failure modes the GitHub REST API produces.

use thiserror::Error;

/// Error surface for GitHub REST API calls.
#[derive(Debug, Error)]
pub enum GitHubError {
    /// Underlying transport failure (DNS, TLS, connection reset, …).
    #[error("github transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// GitHub rate-limited the call (HTTP 403 with `X-RateLimit-Remaining: 0`
    /// or HTTP 429). The `retry_after_secs` is surfaced when the
    /// `Retry-After` header is present.
    #[error("github rate-limited (retry_after_secs={retry_after_secs:?})")]
    RateLimited {
        /// `Retry-After` header value in seconds, if GitHub sent one.
        retry_after_secs: Option<u64>,
    },

    /// GitHub returned a non-2xx status other than rate-limit.
    #[error("github returned HTTP {status}: {body}")]
    HttpStatus {
        /// Raw HTTP status code.
        status: u16,
        /// Truncated response body for diagnostics.
        body: String,
    },

    /// GitHub returned a 401 Unauthorized or the token is invalid.
    #[error("github authentication failed: {0}")]
    Unauthorized(String),
}

impl From<GitHubError> for starter_spi::Error {
    fn from(err: GitHubError) -> Self {
        match &err {
            GitHubError::Unauthorized(_) => starter_spi::Error::Unauthenticated,
            _ => starter_spi::Error::Internal {
                source: Box::new(err),
            },
        }
    }
}
