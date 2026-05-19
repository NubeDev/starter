//! `ClientError` — what every client method returns on failure.

use thiserror::Error;

/// Failures the client surfaces to callers.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Transport-level error (DNS, TLS, connection refused, etc).
    #[error(transparent)]
    Transport(#[from] reqwest::Error),

    /// Server returned a `Problem` body; the field is the parsed
    /// shape if it could be decoded, or `None` for opaque bodies.
    #[error("server error: {message}")]
    Server {
        /// HTTP status code.
        status: u16,
        /// Short human description from the server (or generic).
        message: String,
        /// Original `Problem` body if it was JSON-shaped.
        problem: Option<starter_spi::dto::Problem>,
    },
}
