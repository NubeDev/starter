//! Errors raised loading a kinds pack and dispatching a kind query.
//!
//! Load-time errors (`Load`, `Lint`) surface at boot and abort startup — a
//! malformed pack must never ship. Request-time errors (`Unknown`,
//! `ParamValidation`) map to a 4xx: the caller named a kind that does not exist
//! or supplied params that fail the kind's schema.

use thiserror::Error;

/// Why a kinds pack failed to load or a kind query was rejected.
#[derive(Debug, Error)]
pub enum KindError {
    /// The manifest file could not be read or parsed as YAML.
    #[error("kinds manifest at {path}: {source}")]
    Manifest {
        /// The manifest path being loaded.
        path: String,
        /// The underlying parse/IO error.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A kind's `sql_file` or `params_schema` file could not be read.
    #[error("kind `{kind}` file {path}: {source}")]
    KindFile {
        /// The kind whose file is missing/unreadable.
        kind: String,
        /// The offending file path.
        path: String,
        /// The underlying IO error.
        source: std::io::Error,
    },

    /// A kind's `params_schema` is not a valid JSON Schema document.
    #[error("kind `{kind}` params schema is not valid JSON: {source}")]
    SchemaParse {
        /// The kind whose schema is malformed.
        kind: String,
        /// The serde_json parse error.
        source: serde_json::Error,
    },

    /// A boot-time lint failed: the SQL references an undeclared param, or omits
    /// the mandatory `$caller_tenant_id` predicate on a tenant-scoped table.
    #[error("kind `{kind}` failed validation: {detail}")]
    Lint {
        /// The kind that failed the lint.
        kind: String,
        /// What the lint found.
        detail: String,
    },

    /// Two kinds declared the same reverse-DNS name.
    #[error("duplicate kind name `{0}`")]
    DuplicateName(String),

    /// The caller invoked a kind name no registered kind matches.
    #[error("unknown kind `{0}`")]
    Unknown(String),

    /// Caller params failed validation against the kind's JSON Schema.
    #[error("kind `{kind}` params invalid: {detail}")]
    ParamValidation {
        /// The kind being invoked.
        kind: String,
        /// The first schema violation found.
        detail: String,
    },
}
