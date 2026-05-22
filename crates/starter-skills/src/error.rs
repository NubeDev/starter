//! Structured load-time errors. Every variant carries the path of
//! the offending `SKILL.md` (or the bundle root) so the operator
//! who installed the bundle can fix the problem without grepping
//! through stack traces.
//!
//! Per S-D2 (DOCS/agent/SKILLS.md), an unsupported resource URI
//! scheme **fails at parse time** with
//! [`SkillParseError::UnsupportedResourceScheme`] — no silent skip,
//! no warn-and-continue.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// All load-time failure modes the Phase 1 parser + bundle walker
/// can produce. The variants are intentionally narrow so callers can
/// route on the failure mode (e.g. operator UI vs. CI gate) without
/// string matching.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SkillParseError {
    /// The bundle directory did not contain a `SKILL.md` file.
    #[error("missing SKILL.md in bundle at {bundle_root}")]
    MissingSkillMd {
        /// Bundle directory that was expected to contain `SKILL.md`.
        bundle_root: PathBuf,
    },

    /// Failed to split the `SKILL.md` document into the YAML
    /// frontmatter block and the body. The frontmatter must start
    /// with `---` on the first line and end with a matching `---`
    /// line; anything else is a structural error.
    #[error("malformed frontmatter delimiters in {skill_path}: {reason}")]
    MalformedFrontmatter {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Short reason ("missing opening `---`", "missing closing
        /// `---`", etc.).
        reason: &'static str,
    },

    /// The YAML frontmatter parsed but violated the schema (unknown
    /// key under `deny_unknown_fields`, missing required field,
    /// wrong type, etc.). The source [`serde_yaml::Error`] message
    /// is preserved so the operator sees the YAML cursor.
    #[error("invalid frontmatter in {skill_path}: {source}")]
    InvalidFrontmatter {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Underlying serde_yaml error (line / column / message).
        #[source]
        source: serde_yaml::Error,
    },

    /// The `id:` frontmatter field failed
    /// [`starter_flow_spi::SkillId`] validation (reverse-DNS rule).
    #[error("invalid skill id in {skill_path}: {reason}")]
    InvalidSkillId {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Validator message (from [`starter_flow_spi::node::IdError`]).
        reason: String,
    },

    /// An `allowed_tools` entry failed
    /// [`starter_flow_spi::node::KindId`] validation.
    #[error("invalid allowed_tools entry `{value}` in {skill_path}: {reason}")]
    InvalidAllowedTool {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Raw frontmatter string.
        value: String,
        /// Validator message.
        reason: String,
    },

    /// A `resources:` entry used a URI scheme not in
    /// [`crate::SUPPORTED_RESOURCE_SCHEMES`]. V1 supports `file://`
    /// only (S-D2 locked); broadening is a future-version concern.
    #[error("unsupported resource URI scheme `{scheme}` in {skill_path} (uri: {resource_uri})")]
    UnsupportedResourceScheme {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Raw URI as written in the frontmatter.
        resource_uri: String,
        /// The rejected scheme (`""` if the URI was not parseable
        /// as a `scheme://path` shape).
        scheme: String,
    },

    /// A `resources:` entry pointed at a path that escaped the
    /// bundle root (absolute path, `..` traversal, symlink to
    /// outside the bundle). The Phase 1 walker rejects these at
    /// parse time so the registry never holds an out-of-bundle
    /// reference.
    #[error("resource path `{resource_uri}` escapes bundle in {skill_path}")]
    ResourcePathEscapesBundle {
        /// Path of the offending `SKILL.md`.
        skill_path: PathBuf,
        /// Raw URI as written in the frontmatter.
        resource_uri: String,
    },

    /// I/O error reading the bundle (the `SKILL.md`, a listed
    /// resource, or the bundle directory itself).
    #[error("io error reading {path}: {source}")]
    Io {
        /// Path the walker was reading when the error fired.
        path: PathBuf,
        /// Underlying [`io::Error`].
        #[source]
        source: io::Error,
    },
}
