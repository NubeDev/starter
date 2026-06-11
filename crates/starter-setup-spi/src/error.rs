//! Error type shared across the setup SPI and its backends.

use thiserror::Error;

/// Result alias for setup-layer fallible operations.
pub type SetupResult<T> = Result<T, SetupError>;

/// Errors surfaced by the setup domain: store backends, YAML
/// import/export, and binding/validation checks.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SetupError {
    /// The requested template/run does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// A storage backend failed (DB error, serialization at the
    /// persistence boundary, …).
    #[error("backend failure: {0}")]
    Backend(String),

    /// A YAML envelope could not be parsed or the nested flow body was
    /// malformed.
    #[error("invalid template yaml: {0}")]
    InvalidYaml(String),

    /// The template's flow body failed node-kind / topology validation.
    #[error("invalid flow body: {0}")]
    InvalidBody(String),

    /// Launch input did not match the template's `input_schema`.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A template binding targeted a reserved trusted-identity slot, or
    /// was otherwise structurally invalid (DOCS §9).
    #[error("invalid binding: {0}")]
    InvalidBinding(String),

    /// The principal is not permitted to perform the action (the
    /// setup-layer team check, distinct from the generic authz gate —
    /// DOCS §10).
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// A semver string could not be parsed.
    #[error("invalid version: {0}")]
    InvalidVersion(String),

    /// The run is not in a state that permits this transition (e.g.
    /// resuming a run that is not finished-failed/resumable).
    #[error("invalid run state: {0}")]
    InvalidRunState(String),
}
