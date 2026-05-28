//! The single `Error` enum every starter crate returns up the stack.
//!
//! Variants describe *what went wrong in the domain*, not *what HTTP
//! status to return*. Mapping to transport shapes happens in
//! `starter-server` / `starter-mcp` / etc., never here.

use thiserror::Error;

/// Top-level error type for the starter ecosystem.
///
/// Add a variant when a new failure mode is genuinely distinct — if
/// a caller would handle it differently from existing variants.
/// Otherwise reuse `Internal { source }` and let the message carry
/// the detail.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The requested resource does not exist.
    #[error("not found: {what}")]
    NotFound {
        /// Human description of the missing resource ("user 42",
        /// "config key `auth.issuer`").
        what: String,
    },

    /// Caller-supplied input failed validation.
    #[error("invalid input: {message}")]
    Invalid {
        /// Human description of what was wrong with the input.
        message: String,
    },

    /// Caller is not authenticated (no/expired credentials).
    #[error("unauthenticated")]
    Unauthenticated,

    /// Caller is authenticated but not permitted to perform the action.
    #[error("forbidden")]
    Forbidden,

    /// Resource exists but a precondition / version check failed.
    /// Used for optimistic-locking failures.
    #[error("conflict: {message}")]
    Conflict {
        /// Human description of the conflict.
        message: String,
    },

    /// A downstream dependency failed (DB, network, etc.). The
    /// underlying error is preserved as `source`.
    #[error("internal error")]
    Internal {
        /// The underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// A downstream resource exists but cannot serve the call right
    /// now and may be recoverable via an explicit restart, reconnect,
    /// or retry. Distinct from [`Self::Internal`] because the caller
    /// (and the transport above it) can do something about it —
    /// surface a "restart" affordance, retry with backoff, or
    /// failover. Maps to HTTP 503.
    #[error("unavailable: {message}")]
    Unavailable {
        /// Stable, namespaced identifier of the failure class —
        /// transports key recovery UI off this rather than parsing
        /// `message`. Example: `"extension.supervisor_unavailable"`.
        code: String,
        /// Optional identifier of the unavailable subject (typically
        /// an extension id, worker id, or connection name). When
        /// present, transports can synthesise a recovery URL keyed
        /// on it.
        subject: Option<String>,
        /// Human description of what is unavailable.
        message: String,
    },
}

impl Error {
    /// Convenience constructor for [`Self::Unavailable`] without a
    /// subject. Use [`Self::unavailable_subject`] when the caller has
    /// an id (extension, worker, connection) to attach.
    pub fn unavailable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Unavailable {
            code: code.into(),
            subject: None,
            message: message.into(),
        }
    }

    /// Convenience constructor for [`Self::Unavailable`] with a
    /// subject identifier (typically an extension id).
    pub fn unavailable_subject(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            code: code.into(),
            subject: Some(subject.into()),
            message: message.into(),
        }
    }
}
