//! `BlobError` — the typed failure surface every engine maps onto.
//!
//! # Why these variants, and why this many
//!
//! The trap any "single error per crate" enum falls into is the
//! lazy `Backend(String)` shape, which forces every caller into
//! string-matching to recover an operator-meaningful response.
//! The variants below are the small, real set the source SCOPE
//! enumerates — each one corresponds to a distinct response a
//! consumer would render (or a distinct retry policy a client
//! would apply):
//!
//! - [`BlobError::NotFound`] vs [`BlobError::AlreadyExists`] — 404
//!   vs 409 in HTTP. Different consumer paths.
//! - [`BlobError::Unauthorized`] vs [`BlobError::Forbidden`] —
//!   "you forgot to log in" vs "you logged in but are not
//!   allowed." Collapsing these is a common lint failure; the
//!   SCOPE explicitly forbids it because the operator UX differs.
//! - [`BlobError::PreconditionFailed`] — `If-Match` /
//!   `If-None-Match` failure. Distinguishes a stale-write race
//!   from a permissions error.
//! - [`BlobError::PayloadTooLarge`] — backend rejected on size.
//!   Surface separately so the consumer can guide the user
//!   ("split the upload") rather than retrying.
//! - [`BlobError::Throttled`] with a `retry_after` so the caller
//!   can honour the engine's backpressure instead of guessing.
//! - [`BlobError::Timeout`] — distinct from [`BlobError::Backend`]
//!   so callers can apply a bounded retry without retrying
//!   permanent failures.
//! - [`BlobError::Unsupported`] — the engine knows the operation
//!   but cannot fulfil it (e.g. `copy_server_side` across
//!   distinct backends). **B3** rides on this: the trait
//!   *requires* engines to surface `Unsupported` rather than fall
//!   back to a slower client-side path that silently changes
//!   durability.
//! - [`BlobError::Backend`] is the residual — the typed *and*
//!   string-bag escape hatch for the genuinely backend-specific
//!   thing the consumer cannot do anything with except log. Last
//!   resort, not first reach.
//!
//! Marked `#[non_exhaustive]` so adding a new variant later is
//! semver-additive across the 30+ downstream crates that match on
//! this enum.

use std::time::Duration;

/// Boxed dynamic error used by [`BlobError::Backend`]. Type alias
/// so engines do not have to spell out the long form at every
/// call site.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// The unified failure surface for every [`super::BlobStore`]
/// implementation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BlobError {
    /// The requested [`super::BlobRef`] does not exist (or the
    /// caller has been told it does not, in the case of a
    /// permission-aware engine that hides existence). Maps to
    /// HTTP `404`.
    #[error("blob not found")]
    NotFound,

    /// The caller did not present credentials, or the credentials
    /// are invalid. Maps to HTTP `401`. Distinct from
    /// [`BlobError::Forbidden`] — see module docs.
    #[error("unauthorized")]
    Unauthorized,

    /// The caller authenticated but is not permitted to perform
    /// this operation. Maps to HTTP `403`. Distinct from
    /// [`BlobError::Unauthorized`].
    #[error("forbidden")]
    Forbidden,

    /// A conditional `put` (e.g. `If-None-Match: *`) found the
    /// key already populated. Maps to HTTP `409`.
    #[error("blob already exists")]
    AlreadyExists,

    /// A conditional `put` (e.g. `If-Match: <etag>`) found the
    /// current `Etag` does not match the precondition. Maps to
    /// HTTP `412`. Distinct from [`BlobError::AlreadyExists`]
    /// because the recovery path differs — `AlreadyExists`
    /// implies "pick a different key", `PreconditionFailed`
    /// implies "re-read and retry".
    #[error("precondition failed")]
    PreconditionFailed,

    /// Backend refused the put because the body exceeded its
    /// configured maximum object size. Maps to HTTP `413`.
    #[error("payload too large")]
    PayloadTooLarge,

    /// Backend asked the caller to slow down. `retry_after` is
    /// the engine's hint about when to try again; honour it.
    /// Maps to HTTP `429`.
    #[error("throttled, retry after {retry_after:?}")]
    Throttled {
        /// How long the caller should wait before retrying.
        /// `None` when the engine cannot give a meaningful hint.
        retry_after: Option<Duration>,
    },

    /// Operation exceeded its time budget. Distinct from
    /// [`BlobError::Backend`] because the caller's retry policy
    /// differs.
    #[error("operation timed out")]
    Timeout,

    /// Engine does not support this operation. The B3 escape
    /// hatch — engines surface this rather than silently falling
    /// back to a slower path that changes durability. Example:
    /// `copy_server_side` across distinct backends.
    #[error("operation not supported by this engine")]
    Unsupported,

    /// Residual backend failure that does not fit any of the
    /// typed variants. Carries the originating error so a
    /// consumer's structured logger can record it without losing
    /// the cause chain.
    #[error("blob backend error: {0}")]
    Backend(#[source] BoxError),
}

impl BlobError {
    /// Convenience constructor for [`BlobError::Backend`] from
    /// any concrete error type. Engines reach for this rather
    /// than spelling out the boxing.
    pub fn backend<E>(source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Backend(Box::new(source))
    }
}
