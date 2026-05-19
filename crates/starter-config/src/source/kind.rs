//! Tag identifying which layer a value came from. Used by
//! diagnostic tooling (`starter-cli config dump`) to show
//! *where* a setting was resolved from.

/// Where a config value originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Programmatic default in the consumer's binary.
    Default,
    /// Read from a TOML file.
    File,
    /// Read from an environment variable.
    Env,
    /// Set programmatically after env (CLI flag override).
    Override,
}
