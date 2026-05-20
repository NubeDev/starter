//! Stage 21 — workspace-wide smoke tests for the user-preferences +
//! i18n rollout (DOCS/user/scope/SCOPE.md "Smoke-tests" block).
//!
//! Three structural checks live here, each guarding an invariant
//! that the SCOPE document calls out by name:
//!
//! 1. **Headless appliance keeps working** — a consumer that does NOT
//!    compile in `starter-prefs` / `starter-i18n` (the `examples/minimal`
//!    appliance is the workspace's reference for this posture) still
//!    builds and its dependency closure contains neither crate. No
//!    middleware, no extra routes, no extra migrations leak in via
//!    re-exports or feature toggles.
//!
//! 2. **`starter-prefs` dep gate** — `cargo tree -p starter-prefs
//!    --edges normal` contains `iso_currency` (the Phase 1 dep that
//!    Phase 0 deliberately deferred per SCOPE Decision D-U0.3).
//!
//! 3. **`starter-i18n` dep gate** — `cargo tree -p starter-i18n
//!    --edges normal` contains both `icu_locale_core` (enabled
//!    feature-gated through `starter-spi`) and `sha2` (for catalog
//!    content-hash fingerprinting per Phase 3).
//!
//! The dep-tree gates for `starter-spi` and `starter-flow-spi`
//! already have their own dedicated smoke tests
//! (`smoke_1_no_dep_leakage.rs`) wired to
//! `scripts/check-spi-dep-baseline.sh` and the flow-side equivalent;
//! this file adds the two new crates' gates without duplicating the
//! baseline-file machinery, because their wire shape is "contains the
//! key crate" rather than "byte-for-byte identical to a recorded
//! snapshot".
//!
//! Every test shells out to `cargo` so a single source of truth
//! (`cargo tree` / `cargo build`) drives both CI and the developer
//! loop — Decision D1's "exactly one definition of the canonical
//! command" stance, applied to the smoke layer.

use std::path::PathBuf;
use std::process::Command;

/// `<repo>/crates/smoke-tests` is `CARGO_MANIFEST_DIR` here; the
/// workspace root is two directories up.
fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root above crates/smoke-tests")
        .to_path_buf()
}

/// Run `cargo tree -p <crate> --edges normal --prefix none` from the
/// workspace root and return its stdout as a `String`.
///
/// `--prefix none` produces one crate per line with no indent /
/// re-display markers, which keeps `contains`-style assertions
/// readable and stable across cargo versions.
fn cargo_tree(crate_name: &str) -> String {
    let root = repo_root();
    let output = Command::new("cargo")
        .current_dir(&root)
        .args([
            "tree", "-p", crate_name, "--edges", "normal", "--prefix", "none",
        ])
        .output()
        .unwrap_or_else(|err| panic!("failed to run `cargo tree -p {crate_name}`: {err}"));

    assert!(
        output.status.success(),
        "`cargo tree -p {crate_name}` failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8(output.stdout).expect("cargo tree stdout is utf-8")
}

/// Smoke 1 — the headless appliance (the `examples/minimal` binary,
/// which compiles `starter-server` + `starter-store-sqlite` +
/// `starter-auth-token` + `starter-cli` but not the user-scope
/// crates) does not pull in `starter-prefs` or `starter-i18n`
/// through any transitive edge.
///
/// This is the SCOPE "Headless appliance keeps working" exit gate:
/// the user-scope rollout is purely additive at the workspace level,
/// not a forced platform dependency.
#[test]
fn headless_appliance_does_not_compile_prefs_or_i18n() {
    let tree = cargo_tree("starter-minimal");

    for forbidden in ["starter-prefs", "starter-i18n"] {
        // `cargo tree --prefix none` prints `<crate> v<version>`
        // entries one per line — match on the prefix to avoid a
        // false positive against e.g. a hypothetical
        // `starter-prefs-something` rename.
        let leaked = tree.lines().any(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix(forbidden)
                .is_some_and(|rest| rest.starts_with(' '))
        });
        assert!(
            !leaked,
            "starter-minimal pulled in `{forbidden}` — the headless \
             appliance must build without user-scope crates. \
             Full cargo tree:\n{tree}"
        );
    }
}

/// Smoke 1b — the headless appliance actually builds. The dep-tree
/// check above is necessary but not sufficient; a workspace can
/// have a clean tree and still fail to compile because of a
/// downstream feature conflict. A bare `cargo build -p
/// starter-minimal` is the cheapest possible confirmation that the
/// "binary built without user-scope crates" claim survives a real
/// compile.
#[test]
fn headless_appliance_builds() {
    let root = repo_root();
    let output = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "-p", "starter-minimal", "--quiet"])
        .output()
        .expect("run `cargo build -p starter-minimal`");

    assert!(
        output.status.success(),
        "`cargo build -p starter-minimal` failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Smoke 2 — `starter-prefs`' dep tree contains `iso_currency`.
///
/// Phase 0's Decision D-U0.3 deliberately kept `iso_currency` off
/// `starter-spi` so that the workspace's leaf platform crate did
/// not pay currency-table cost. The currency data lands here at
/// Phase 1; if a future refactor accidentally moves the dep back
/// up the tree (or drops it entirely) this test catches it.
#[test]
fn starter_prefs_dep_tree_contains_iso_currency() {
    let tree = cargo_tree("starter-prefs");
    assert!(
        tree.lines()
            .any(|line| line.trim().starts_with("iso_currency ")),
        "starter-prefs is missing `iso_currency` — Phase 0 D-U0.3 \
         requires it to land here, not on starter-spi. Tree:\n{tree}"
    );
}

/// Smoke 3 — `starter-i18n`'s dep tree contains both
/// `icu_locale_core` and `sha2`, the two non-trivial deps the
/// Phase 3 scope locks in.
///
/// `icu_locale_core` is feature-gated on `starter-spi` (D-0.1 / D-0.2
/// keep it default-off there) but `starter-i18n` turns the feature
/// on for BCP-47 negotiation; `sha2` ships the catalog fingerprint
/// (Phase 3 "Decisions": 16-char lowercase-hex prefix). Both are
/// load-bearing for the documented behaviour, so dropping either
/// would silently break the workspace contract.
#[test]
fn starter_i18n_dep_tree_contains_icu_locale_core_and_sha2() {
    let tree = cargo_tree("starter-i18n");
    for required in ["icu_locale_core ", "sha2 "] {
        assert!(
            tree.lines().any(|line| line.trim().starts_with(required)),
            "starter-i18n is missing `{}` — Phase 3 requires it. \
             Tree:\n{tree}",
            required.trim_end()
        );
    }
}
