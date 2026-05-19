//! Config-loading error type. Maps to `starter_spi::Error::Internal`
//! at the boundary; kept distinct here so callers in `main.rs` can
//! print a useful diagnostic before tracing is even initialised.

use thiserror::Error;

/// Failure modes when loading config.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file at `path` could not be read or parsed.
    #[error("failed to read config file {path}: {source}")]
    File {
        /// Path that failed.
        path: String,
        /// Underlying I/O or parse error.
        #[source]
        source: figment::Error,
    },

    /// Final merged config could not be deserialized into the
    /// consumer's struct.
    #[error("config shape mismatch: {0}")]
    Shape(#[from] figment::Error),
}
