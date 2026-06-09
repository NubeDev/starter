//! Errors raised loading a datasource-kinds pack and validating a connector
//! config against its declared schema.
//!
//! Load-time errors (`Manifest`, `KindFile`, `SchemaParse`, `Lint`,
//! `DuplicateName`) surface at boot and abort startup — a malformed pack must
//! never ship. Request-time errors (`Unknown`, `ConfigValidation`) map to a 4xx:
//! the caller named a datasource-kind that does not exist or supplied config that
//! fails the kind's schema.

use thiserror::Error;

/// Why a datasource-kinds pack failed to load or a connector config was rejected.
#[derive(Debug, Error)]
pub enum DatasourceKindError {
    /// The manifest file could not be read or parsed as YAML.
    #[error("datasource-kinds manifest at {path}: {source}")]
    Manifest {
        /// The manifest path being loaded.
        path: String,
        /// The underlying parse/IO error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A kind's `config_schema` file could not be read.
    #[error("datasource-kind `{kind}` file {path}: {source}")]
    KindFile {
        /// The datasource-kind whose file is missing/unreadable.
        kind: String,
        /// The offending file path.
        path: String,
        /// The underlying IO error.
        source: std::io::Error,
    },

    /// A kind's `config_schema` is not a valid JSON Schema document.
    #[error("datasource-kind `{kind}` config schema is not valid JSON: {source}")]
    SchemaParse {
        /// The datasource-kind whose schema is malformed.
        kind: String,
        /// The serde_json parse error.
        source: serde_json::Error,
    },

    /// A boot-time lint failed: a declared `secret_field` is not a property of the
    /// config schema, or the schema is not an object schema.
    #[error("datasource-kind `{kind}` failed validation: {detail}")]
    Lint {
        /// The datasource-kind that failed the lint.
        kind: String,
        /// What the lint found.
        detail: String,
    },

    /// Two datasource-kinds declared the same name.
    #[error("duplicate datasource-kind name `{0}`")]
    DuplicateName(String),

    /// The caller invoked a datasource-kind name no registered kind matches.
    #[error("unknown datasource-kind `{0}`")]
    Unknown(String),

    /// A connector config failed validation against the kind's config schema.
    #[error("datasource-kind `{kind}` config invalid: {detail}")]
    ConfigValidation {
        /// The datasource-kind being configured.
        kind: String,
        /// The first schema violation found.
        detail: String,
    },
}
