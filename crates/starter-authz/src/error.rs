//! Crate-local error type. Engine evaluation does not return
//! `Result` — it returns [`starter_spi::authz::Decision`]. This
//! error is reserved for setup-time failures: config parse, double
//! resource registration, invalid condition expressions.

use thiserror::Error;

/// Setup-time errors. Runtime authorization decisions go through
/// [`starter_spi::authz::Decision`], not `Result`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// TOML parse / IO failure while loading a policy file.
    #[error("authz config: {0}")]
    Config(String),

    /// A condition expression failed to parse at rule load time.
    #[error("authz condition `{expr}` invalid: {reason}")]
    Condition {
        /// The offending expression.
        expr: String,
        /// Human-readable failure cause.
        reason: String,
    },

    /// Two registrations for the same resource kind. This is a
    /// panic in [`crate::StaticRegistry::register`] (per SCOPE.md
    /// "two extensions register the same kind" — loud failure
    /// beats silent shadowing) and only surfaces as a `Result`
    /// from the non-panicking [`crate::StaticRegistry::try_register`].
    #[error("authz resource `{kind}` already registered")]
    DuplicateResource {
        /// The kind that was registered twice.
        kind: String,
    },
}

/// `Result` alias for crate-local setup operations.
pub type Result<T> = std::result::Result<T, Error>;
