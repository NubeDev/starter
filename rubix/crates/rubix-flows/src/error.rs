//! Loader error type.

use starter_flow_spi::node::IdError;
use thiserror::Error;

/// Errors surfaced by the YAML loader pipeline.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    /// A bundled file's bytes were not valid UTF-8.
    #[error("flow `{path}`: not valid UTF-8: {source}")]
    Utf8 {
        path: String,
        #[source]
        source: std::str::Utf8Error,
    },
    /// `serde_yaml` rejected the file shape.
    #[error("flow `{path}`: YAML shape: {source}")]
    Yaml {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    /// The flow id, node id, or kind id failed reverse-DNS validation.
    #[error("flow `{path}`: id `{id}`: {source}")]
    Id {
        path: String,
        id: String,
        #[source]
        source: IdError,
    },
    /// The flow body had zero nodes.
    #[error("flow `{path}`: must declare at least one node")]
    EmptyBody { path: String },
    /// `config.allowed_tools` was present but malformed (not a
    /// sequence of strings).
    #[error("node `{node}`: {message}")]
    AllowedTools { node: String, message: String },
    /// `config.tools` was present but malformed (not a sequence of
    /// strings). `tools` is the CLI built-in restriction — distinct
    /// from `allowed_tools` (the MCP-bridged surface).
    #[error("node `{node}`: {message}")]
    Tools { node: String, message: String },
}
