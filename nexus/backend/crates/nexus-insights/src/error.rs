//! Structured insight errors, safe to surface to a tenant.
//!
//! The three phases a tenant cares about are distinct variants: a `Compile`
//! error is the tenant's script syntax; a `Runtime` error is their logic failing
//! against the data (a missing column, a bad argument); a `LimitExceeded` error
//! is the sandbox stopping a script that ran too long, looped forever, or built
//! something too large. None carries an internal pointer or a panic — the whole
//! point of the sandbox is that a hostile script produces a clean error, never a
//! crash across the script↔engine edge.

use thiserror::Error;

/// A failure running an insight. Each variant maps to one tenant-facing cause.
#[derive(Debug, Error)]
pub enum InsightError {
    /// The script did not compile — a syntax error in the tenant's Rhai. The
    /// string is Rhai's own position-annotated message.
    #[error("insight compile error: {0}")]
    Compile(String),

    /// The script compiled but failed while running: a primitive rejected its
    /// arguments (unknown column, empty frame) or the script raised an error.
    #[error("insight runtime error: {0}")]
    Runtime(String),

    /// A sandbox limit tripped: too many operations, call depth, an oversized
    /// string/array, or the wall-clock deadline fired. The script is stopped and
    /// this is returned — it is the designed outcome for a pathological script.
    #[error("insight limit exceeded: {0}")]
    LimitExceeded(String),

    /// The vectorized engine itself failed converting or computing a frame. Kept
    /// separate from `Runtime` so an engine bug is not mistaken for tenant error.
    #[error("insight engine error: {0}")]
    Engine(String),
}

/// Convenience alias for insight results.
pub type InsightResult<T> = Result<T, InsightError>;
