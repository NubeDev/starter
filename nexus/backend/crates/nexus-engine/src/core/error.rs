//! Engine error type for the native pipeline core.
//!
//! Distinct variants per pipeline phase so a failure points at the node that
//! produced it (build/source/processor/sink) rather than a flat string. A run
//! that ends because its cancellation token fired is not an error — that is
//! reported through `RunOutcome::Cancelled`, never here.

use thiserror::Error;

/// A failure raised while building or running a pipeline. Each variant names
/// the phase that failed so callers and logs can attribute it without parsing.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A node config was malformed, or named a builder the registry does not
    /// know. The string is the registry/parse message.
    #[error("pipeline build failed: {0}")]
    Build(String),

    /// A source raised an error while reading. The pipeline stops and propagates.
    #[error("source read failed: {0}")]
    Source(String),

    /// A processor raised an error transforming a batch. The pipeline stops.
    #[error("processor failed: {0}")]
    Processor(String),

    /// A sink raised an error writing or closing. The pipeline stops.
    #[error("sink write failed: {0}")]
    Sink(String),
}

/// Convenience alias for engine results.
pub type EngineResult<T> = Result<T, EngineError>;
