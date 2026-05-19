//! Workspace-wide error type.
//!
//! Every crate in `starter-extensions` returns this enum at its outer API
//! boundary so consumers see one shape regardless of which flavour
//! (builtin, wasm, process) raised the failure. SCOPE.md "What each crate /
//! package owns" lists the variants explicitly under `starter-ext-spi`.

use serde::{Deserialize, Serialize};

/// Convenience alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All known failure categories for the extension substrate.
///
/// Variants are stable; adding a category is additive within the crate
/// major. Each variant carries a free-form message — the kernel does not
/// enumerate every possible reason because that explodes combinatorially
/// across adapters (REST shapes differ from gRPC shapes differ from MCP
/// shapes). What the kernel guarantees is the *category*, so adapters can
/// map to their transport's error model uniformly.
#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum Error {
    /// The `block.yaml` failed to parse or violated `deny_unknown_fields`
    /// (typo in a key, wrong type for a value, unknown field, …).
    #[error("manifest: {0}")]
    Manifest(String),

    /// The manifest parsed but failed a semantic check (R4 namespace
    /// ownership, R6 capability compatibility, schema version mismatch,
    /// duplicate id, …).
    #[error("validation: {0}")]
    Validation(String),

    /// Failure to spawn or initialise a flavour-specific runtime: process
    /// supervisor could not exec the child, WASI host could not link the
    /// component, builtin registry could not look up the static table.
    #[error("spawn: {0}")]
    Spawn(String),

    /// Failure on the host↔extension transport: JSON-RPC framing error,
    /// stdio pipe closed mid-message, malformed envelope, missing
    /// `jsonrpc` field, …
    #[error("transport: {0}")]
    Transport(String),

    /// A capability check refused a call. Used by adapters when an
    /// extension's request references a capability it did not declare
    /// (process-flavour advisory enforcement; WASM rejects at link time).
    #[error("capability: {0}")]
    Capability(String),

    /// The extension's own handler returned an error. Wrapped so the host
    /// can distinguish "extension code failed" from "substrate failed".
    /// Adapters surface this as a normal application error to the caller.
    #[error("extension: {0}")]
    ExtensionInternal(String),
}

impl Error {
    /// Convenience constructor for `Manifest(...)`.
    pub fn manifest(msg: impl Into<String>) -> Self {
        Self::Manifest(msg.into())
    }
    /// Convenience constructor for `Validation(...)`.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    /// Convenience constructor for `Spawn(...)`.
    pub fn spawn(msg: impl Into<String>) -> Self {
        Self::Spawn(msg.into())
    }
    /// Convenience constructor for `Transport(...)`.
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }
    /// Convenience constructor for `Capability(...)`.
    pub fn capability(msg: impl Into<String>) -> Self {
        Self::Capability(msg.into())
    }
    /// Convenience constructor for `ExtensionInternal(...)`.
    pub fn extension_internal(msg: impl Into<String>) -> Self {
        Self::ExtensionInternal(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_wire_form() {
        let e = Error::Capability("missing http_out".to_string());
        let j = serde_json::to_value(&e).unwrap();
        assert_eq!(j["kind"], "capability");
        assert_eq!(j["message"], "missing http_out");
        let back: Error = serde_json::from_value(j).unwrap();
        assert_eq!(back, e);
    }
}
