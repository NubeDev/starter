//! Smoke test 1 — no dep leakage via `starter-spi`.
//!
//! Wraps `scripts/check-spi-dep-baseline.sh` so a `cargo test` run
//! catches the same drift CI catches. CI is the authority (see the
//! `spi-dep-baseline` job in `.github/workflows/ci.yml`); this is the
//! developer-loop convenience.
//!
//! The test shells out to the canonical script rather than reimplementing
//! the diff in Rust so there is exactly one definition of "normalised
//! `cargo tree` output" — Decision D1 anticipates the normalisation
//! recipe will evolve and we don't want it drifting between the script
//! and a Rust copy.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/crates/smoke-tests`; pop twice.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root above crates/smoke-tests")
        .to_path_buf()
}

#[test]
fn starter_spi_dep_baseline_matches() {
    let root = repo_root();
    let script = root.join("scripts/check-spi-dep-baseline.sh");
    assert!(
        script.exists(),
        "missing baseline-check script at {}",
        script.display()
    );

    let output = Command::new("bash")
        .arg(&script)
        .output()
        .expect("execute scripts/check-spi-dep-baseline.sh");

    if !output.status.success() {
        panic!(
            "starter-spi dep baseline drifted.\n\
             stdout:\n{}\n\
             stderr:\n{}\n\
             \n\
             If starter-spi itself changed direct deps, regenerate the\n\
             baseline in the same commit:\n\
             \n\
             \tscripts/check-spi-dep-baseline.sh --update\n\
             \n\
             Otherwise a provider crate's deps have leaked — fix the\n\
             leak, do not update the baseline (Decision D1).",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
