//! How a pipeline run ended, distinguishing a clean finish from a cancellation.
//!
//! A finite source returning `None` ends a run as [`RunOutcome::Completed`]; a
//! fired cancellation token ends it as [`RunOutcome::Cancelled`] after the
//! in-flight batch drains and the sink closes. Both are successes — an error
//! comes back as `Err(EngineError)`, never as an outcome variant.

/// The terminal state of a successful [`super::pipeline::Pipeline::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// The source signalled end-of-stream and every batch reached the sink,
    /// which was then closed. This is the only terminal state a finite pipeline
    /// reaches on its own.
    Completed,

    /// The cancellation token fired. The pipeline stopped reading, drained the
    /// batch already in flight, and closed the sink. Reached by infinite
    /// pipelines (live, flows) and by any run stopped early.
    Cancelled,
}

impl RunOutcome {
    /// Whether the run was cancelled rather than completing on its own.
    pub fn is_cancelled(self) -> bool {
        matches!(self, RunOutcome::Cancelled)
    }
}
