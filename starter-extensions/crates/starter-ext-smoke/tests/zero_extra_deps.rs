//! SCOPE smoke: "Extension author has zero starter-workspace deps".
//!
//! Each `hello-*` example's `Cargo.toml` declares only the
//! starter-extensions surface the SDK promises to extension authors:
//!
//!   - `starter-ext-sdk`        (mandatory, one flavour feature)
//!   - `serde_json`             (handler payloads)
//!   - `tokio`                  (process flavour only — for the entry-point glue)
//!
//! Any other `starter-*` / `starter-ext-*` crate appearing as a *direct*
//! dependency of the example crate is a regression — it means the SDK
//! is leaking host-internal types out of its public API and the
//! extension author would have to import them to compile their crate.
//!
//! `hello-cli` is intentionally exempt: its `[[bin]]` half is the host
//! side of the CLI demo (it wires `starter-cli` + `starter-ext-cli` +
//! `starter-ext-host` together to drive the contributed subcommand).
//! The extension half lives in `hello_cli` (the lib target) and is
//! covered indirectly by the audit below — once we tighten the example
//! to a separate crate this list grows.

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

/// Whitelist of dependency names an extension author is *allowed* to
/// list. Anything else fails the audit.
const ALLOWED: &[&str] = &[
    // The single SDK entry point.
    "starter-ext-sdk",
    // Serde for handler params/return values.
    "serde_json",
    // Tokio for the process-flavour entry-point glue. The SDK's
    // `register_process_main!` expands to `#[tokio::main]`; the example
    // has to enable it.
    "tokio",
];

/// Parsed shape of the `[dependencies]` table of a `Cargo.toml`. We only
/// care about the *keys* — the values can be string-shorthand, table,
/// or workspace-inherit.
#[derive(serde::Deserialize)]
struct ManifestShape {
    #[serde(default)]
    dependencies: toml::value::Table,
}

fn dep_keys(manifest_path: &Path) -> BTreeSet<String> {
    let text = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|e| panic!("read {manifest_path:?}: {e}"));
    let parsed: ManifestShape = toml::from_str(&text)
        .unwrap_or_else(|e| panic!("parse {manifest_path:?}: {e}"));
    parsed.dependencies.keys().cloned().collect()
}

#[test]
fn hello_builtin_has_only_allowed_deps() {
    let path = workspace_root().join("examples/hello-builtin/Cargo.toml");
    let deps = dep_keys(&path);
    for name in &deps {
        assert!(
            ALLOWED.contains(&name.as_str()),
            "hello-builtin pulled in {name:?} — only {ALLOWED:?} are permitted \
             (SCOPE 'Extension author has zero starter-workspace deps' test)"
        );
    }
    assert!(deps.contains("starter-ext-sdk"));
}

#[test]
fn hello_process_has_only_allowed_deps() {
    let path = workspace_root().join("examples/hello-process/Cargo.toml");
    let deps = dep_keys(&path);
    for name in &deps {
        assert!(
            ALLOWED.contains(&name.as_str()),
            "hello-process pulled in {name:?} — only {ALLOWED:?} are permitted"
        );
    }
    assert!(deps.contains("starter-ext-sdk"));
    assert!(
        deps.contains("tokio"),
        "process flavour needs tokio for the entry-point glue"
    );
}

#[test]
fn hello_wasm_has_only_allowed_deps() {
    let path = workspace_root().join("examples/hello-wasm/Cargo.toml");
    let deps = dep_keys(&path);
    for name in &deps {
        assert!(
            ALLOWED.contains(&name.as_str()),
            "hello-wasm pulled in {name:?} — only {ALLOWED:?} are permitted"
        );
    }
    assert!(deps.contains("starter-ext-sdk"));
}
