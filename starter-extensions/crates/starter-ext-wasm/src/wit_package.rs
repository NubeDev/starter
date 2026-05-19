//! The embedded `starter:extension@0.1.0` WIT package.
//!
//! Embedding the WIT bytes (rather than reading them from disk at host
//! startup) is a deliberate R3/R7 mirror: the package is the wasm-flavour
//! twin of `block.yaml` and the JSON-RPC envelope — one source of truth,
//! checked at the extension's compile time, never templated at runtime.
//! `WasmHost` does not currently parse the bytes (component-side
//! validation already happens inside wasmtime); they live here so the
//! admin endpoint can serve them verbatim and so a future minor of the
//! package surfaces an unambiguous version diff.

/// Package name, matching the `package` declaration in
/// `wit/starter-extension.wit`.
pub const WIT_PACKAGE_NAME: &str = "starter:extension";

/// Package version. Bumping this is a substrate-major event — every
/// wasm-flavour extension in the wild needs to be recompiled against the
/// new package's bindings. v0.1.x is additive within `0.1`.
pub const WIT_PACKAGE_VERSION: &str = "0.1.0";

/// Raw `starter:extension@0.1.0` WIT bytes embedded at this crate's
/// compile time.
pub const WIT_PACKAGE: &str = include_str!("../wit/starter-extension.wit");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_package_carries_expected_header() {
        assert!(WIT_PACKAGE.contains(&format!(
            "package {WIT_PACKAGE_NAME}@{WIT_PACKAGE_VERSION};"
        )));
    }

    #[test]
    fn stream_notification_imports_are_reserved() {
        // SCOPE.md "stream-notification import names reserved alongside"
        // — the four post-R13 notifications must exist in the package
        // even when v0.1's host wires them as no-ops, so a future minor
        // adds the matching guest hooks without bumping the major.
        for name in ["stream-event", "stream-end", "stream-error", "stream-cancel"] {
            assert!(
                WIT_PACKAGE.contains(name),
                "expected {name:?} import reserved in the WIT package"
            );
        }
    }

    #[test]
    fn dispatch_tool_export_is_declared() {
        // The single mandatory export every wasm-flavour extension must
        // implement. If this disappears, every existing extension breaks
        // at instantiation — `assert!` here is the canary.
        assert!(WIT_PACKAGE.contains("dispatch-tool"));
    }
}
