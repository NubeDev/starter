//! SCOPE smoke: "One source, three flavours" — mutually-exclusive
//! cargo features.
//!
//! The actual three-way compile is performed by the CI matrix
//! (`.github/workflows/starter-extensions.yml`) — at host-test time we
//! can only inspect the *manifest* of each `hello-*` example and assert
//! that:
//!
//!   - exactly one of `builtin` / `wasm` / `process` is requested as a
//!     feature of `starter-ext-sdk`, and
//!   - the Rust source bodies (modulo the entry-point macro and the
//!     module shape forced by each flavour's expected lib/bin layout)
//!     are byte-identical at the trait-impl block.
//!
//! The SDK's own `lib.rs` carries the `compile_error!` and the
//! duplicate-`#[no_mangle]` linker trap that enforce mutual exclusion
//! when the feature flags are wrong (see
//! `crates/starter-ext-sdk/src/lib.rs:39`). The CI matrix exercises
//! that trap; this file pins the *example* side.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Pull the feature set the example enables on its `starter-ext-sdk`
/// dependency.
fn sdk_features(manifest_path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(manifest_path).unwrap();
    let parsed: toml::Value = toml::from_str(&text).unwrap();
    let deps = parsed
        .get("dependencies")
        .and_then(|v| v.as_table())
        .unwrap_or_else(|| panic!("{manifest_path:?} has no [dependencies]"));
    let entry = deps
        .get("starter-ext-sdk")
        .unwrap_or_else(|| panic!("{manifest_path:?} does not depend on starter-ext-sdk"));
    let table = entry
        .as_table()
        .unwrap_or_else(|| panic!("starter-ext-sdk dep must be a table (workspace = true ..)"));
    let features = table
        .get("features")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    features
        .into_iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

fn assert_exactly_one_flavour(manifest_path: &Path, expected: &str) {
    let features: BTreeSet<String> = sdk_features(manifest_path).into_iter().collect();
    let flavours: BTreeSet<&str> = ["builtin", "wasm", "process"]
        .iter()
        .copied()
        .filter(|f| features.contains(*f))
        .collect();
    assert_eq!(
        flavours.len(),
        1,
        "{manifest_path:?} should request exactly one flavour feature on starter-ext-sdk; \
         got {flavours:?}. SCOPE R1: builtin/wasm/process are mutually exclusive."
    );
    assert!(
        flavours.contains(expected),
        "{manifest_path:?} should select {expected:?}; got {flavours:?}"
    );
}

#[test]
fn hello_builtin_selects_only_builtin() {
    assert_exactly_one_flavour(
        &workspace_root().join("examples/hello-builtin/Cargo.toml"),
        "builtin",
    );
}

#[test]
fn hello_process_selects_only_process() {
    assert_exactly_one_flavour(
        &workspace_root().join("examples/hello-process/Cargo.toml"),
        "process",
    );
}

#[test]
fn hello_wasm_selects_only_wasm() {
    assert_exactly_one_flavour(
        &workspace_root().join("examples/hello-wasm/Cargo.toml"),
        "wasm",
    );
}

/// The `impl HelloToolHandlers for Hello` body must be code-identical
/// across flavours — comments may diverge to point at the right
/// flavour's surrounding glue, but the actual handler statements must
/// not. Only the entry-point macro and the file-level boilerplate may
/// differ — that is the entire SCOPE R1 budget.
#[test]
fn hello_trait_impl_body_is_byte_identical_across_flavours() {
    let builtin =
        std::fs::read_to_string(workspace_root().join("examples/hello-builtin/src/lib.rs"))
            .unwrap();
    let process =
        std::fs::read_to_string(workspace_root().join("examples/hello-process/src/main.rs"))
            .unwrap();
    let wasm =
        std::fs::read_to_string(workspace_root().join("examples/hello-wasm/src/lib.rs")).unwrap();

    fn extract_impl_body(src: &str) -> String {
        let start = src
            .find("impl HelloToolHandlers for Hello")
            .expect("trait impl present");
        // Find the matching closing brace by counting depth.
        let bytes = src.as_bytes();
        let mut depth = 0i32;
        let mut end = None;
        for (i, b) in bytes[start..].iter().enumerate() {
            match *b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end.expect("balanced braces");
        // Drop comment-only lines so the human-readable rationale can
        // differ per flavour while the code stays identical.
        src[start..end]
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    let b = extract_impl_body(&builtin);
    let p = extract_impl_body(&process);
    let w = extract_impl_body(&wasm);
    assert_eq!(
        b, p,
        "builtin and process trait impls must be byte-identical"
    );
    assert_eq!(b, w, "builtin and wasm trait impls must be byte-identical");
}
