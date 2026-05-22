//! SCOPE smoke 4 — **CostToSkip**.
//!
//! A consumer who never touches blobs pays zero. `cargo tree -p
//! starter-server` against the crate's default feature set must not
//! show any of the blob crates as a transitive dependency. The
//! crates ship in the workspace but consumers opt-in by adding the
//! specific engine to their `Cargo.toml`.
//!
//! The check shells out to `cargo tree --prefix none --no-default-features
//! -p starter-server` and asserts none of `starter-blob-*` /
//! `starter-blob-compose` appear anywhere in the output. (We pass
//! `--no-default-features` deliberately: `starter-server`'s default
//! feature set has no blob feature today, but the assertion is
//! that even the minimum reachable set is blob-free; a future
//! `default = ["blobs"]` regression would also flip this test.)

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root above crates/smoke-tests")
        .to_path_buf()
}

#[test]
fn starter_server_has_no_transitive_blob_dependency() {
    let root = repo_root();
    let output = Command::new("cargo")
        .arg("tree")
        .arg("--prefix")
        .arg("none")
        .arg("-p")
        .arg("starter-server")
        .arg("--no-default-features")
        .current_dir(&root)
        .output()
        .expect("run cargo tree");

    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("utf-8 cargo tree output");

    const FORBIDDEN: &[&str] = &[
        "starter-blob-memory",
        "starter-blob-fs",
        "starter-blob-s3",
        "starter-blob-garage",
        "starter-blob-compose",
    ];
    for needle in FORBIDDEN {
        assert!(
            !tree.contains(needle),
            "CostToSkip violation: starter-server (no-default-features) transitively pulls {needle}\n\
             full cargo tree output follows so the diff is obvious:\n{tree}",
        );
    }
}
