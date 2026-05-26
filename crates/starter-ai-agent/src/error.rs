//! Error surface returned by [`crate::AgentLoop::run`].

use thiserror::Error;

/// Errors surfaced by the agent loop. Upstream provider failures
/// flow through [`AgentError::Runner`]; per-tool failures through
/// [`AgentError::Tool`]; model-requested tools the [`crate::ToolSet`]
/// does not carry through [`AgentError::UnknownTool`]; mis-shaped
/// runner output through [`AgentError::Unparseable`].
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum AgentError {
    /// Provider-side failure surfaced by the runner.
    #[error("runner: {0}")]
    Runner(String),
    /// A tool the model requested executed but returned an error.
    #[error("tool `{name}`: {message}")]
    Tool {
        /// Tool name the model asked for.
        name: String,
        /// Stringified error from the tool's `invoke`.
        message: String,
    },
    /// The model asked for a tool the loop's [`crate::ToolSet`] does
    /// not carry.
    #[error("unknown tool `{0}`")]
    UnknownTool(String),
    /// The runner returned a shape the loop could not interpret.
    #[error("unparseable runner output: {0}")]
    Unparseable(String),
}
