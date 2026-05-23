//! Errors raised by the rubix client surface.

/// Errors that surface from rubix client methods.
#[derive(Debug, thiserror::Error)]
pub enum RubixClientError {
    /// Underlying HTTP / serde failure from reqwest.
    #[error("transport: {0}")]
    Transport(#[from] reqwest::Error),

    /// Server returned a non-success status with an error body.
    #[error("server error: {status}")]
    Server {
        /// HTTP status code.
        status: u16,
    },
}
