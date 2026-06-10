//! One error type for the whole facade. Both layers fail in similar shapes
//! (auth, transport, the provider rejecting the request), so we normalise to a
//! single enum and keep the underlying error text for diagnosis.

use thiserror::Error;

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// Credentials missing or rejected by the provider.
    #[error("auth: {0}")]
    Auth(String),

    /// The requested model/agent alias could not be resolved to a concrete one.
    #[error("unknown model or alias: {0}")]
    UnknownModel(String),

    /// Network / transport failure talking to the provider or agent CLI.
    #[error("transport: {0}")]
    Transport(String),

    /// The provider accepted the call but returned an error response.
    #[error("provider: {0}")]
    Provider(String),

    /// A capability was used that the active build does not include
    /// (e.g. calling `agent()` without the `agent` feature).
    #[error("capability not enabled: {0}")]
    Unsupported(&'static str),
}
