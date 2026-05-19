//! Phase 2 dep-tree gates from DOCS/flow/scope/SCOPE.md, landed as
//! an automated integration test so future regressions break CI
//! immediately. The original starter-flow-engine job (merged as
//! PR #5) only documented these gates as manual-shell-grep steps;
//! the catch-up job stage 7 turns them into a real test.
//!
//! Gates asserted here, all by shelling out to `cargo tree` from
//! the workspace root resolved from `CARGO_MANIFEST_DIR`:
//!
//! - `cargo tree -p starter-flow         --edges normal` contains
//!   zero `adk-rust` matches.
//! - `cargo tree -p starter-flow-nodes   --edges normal` contains
//!   zero `adk-rust` matches.
//! - `cargo tree -p starter-flow-surfaces --edges normal` contains
//!   zero `adk-rust` matches (defensive — the same workspace policy
//!   applies to every flow crate).
//! - `cargo tree -p starter-flow-spi     --edges normal` diffed
//!   against `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt`
//!   is empty (Phase 1 baseline holds; any new dep added to the
//!   SPI crate must be explicitly re-baselined).
//! - None of the four flow crates depend (path or otherwise) on the
//!   Phase 3 surface crates: `starter-mcp`, `starter-server`,
//!   `starter-cli`. Phase 3 wiring runs in the opposite direction.
//!
//! Worktree-absolute paths in `cargo tree` output are stripped
//! before diffing the SPI baseline so this test is stable across
//! worktree relocations (job-XXX hash in the path differs every
//! session).

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/starter-flow; go two up.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("workspace root resolvable from CARGO_MANIFEST_DIR")
}

fn cargo_tree(pkg: &str) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "-p", pkg, "--edges", "normal"])
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| panic!("spawn cargo tree -p {pkg}: {e}"));
    assert!(
        output.status.success(),
        "cargo tree -p {pkg} failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree stdout is utf-8")
}

/// Strip the per-session worktree prefix so baseline diffs are
/// reproducible. Any absolute path that contains
/// `/worktrees/job-XXX/` is collapsed to start at `<WORKTREE>/`.
fn normalise_worktree_paths(s: &str) -> String {
    let needle = "/worktrees/job-";
    s.lines()
        .map(|line| match line.find(needle) {
            None => line.to_string(),
            Some(idx) => {
                // Find the `/` that closes the `job-XXX` segment.
                let after = idx + needle.len();
                let close = match line[after..].find('/') {
                    Some(p) => after + p + 1,
                    None => return line.to_string(),
                };
                // Walk back to the start of the absolute path on
                // this line (the first `/` we can see before idx).
                // Cargo tree always renders an absolute path as a
                // single contiguous run of path chars after the
                // tree glyphs, so the last `/` before idx that is
                // preceded by another path char or by the start of
                // the path run works.
                let path_start = line[..idx].rfind(' ').map(|p| p + 1).unwrap_or(0);
                let mut out = String::with_capacity(line.len());
                out.push_str(&line[..path_start]);
                out.push_str("<WORKTREE>/");
                out.push_str(&line[close..]);
                out
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn starter_flow_tree_contains_no_adk_rust() {
    let tree = cargo_tree("starter-flow");
    assert!(
        !tree.contains("adk-rust"),
        "starter-flow dep tree must not contain `adk-rust`:\n{tree}"
    );
}

#[test]
fn starter_flow_nodes_tree_contains_no_adk_rust() {
    let tree = cargo_tree("starter-flow-nodes");
    assert!(
        !tree.contains("adk-rust"),
        "starter-flow-nodes dep tree must not contain `adk-rust`:\n{tree}"
    );
}

#[test]
fn starter_flow_surfaces_tree_contains_no_adk_rust() {
    let tree = cargo_tree("starter-flow-surfaces");
    assert!(
        !tree.contains("adk-rust"),
        "starter-flow-surfaces dep tree must not contain `adk-rust`:\n{tree}"
    );
}

#[test]
fn starter_flow_spi_baseline_holds() {
    let baseline_path = workspace_root()
        .join("DOCS")
        .join("flow")
        .join("scope")
        .join("starter-flow-spi-deps.baseline.txt");
    let baseline = std::fs::read_to_string(&baseline_path)
        .unwrap_or_else(|e| panic!("read baseline {}: {e}", baseline_path.display()));
    let tree = cargo_tree("starter-flow-spi");

    let baseline_n = normalise_worktree_paths(&baseline);
    let tree_n = normalise_worktree_paths(&tree);
    let baseline_n = baseline_n.trim_end();
    let tree_n = tree_n.trim_end();

    if baseline_n != tree_n {
        // Surface a unified-ish diff inline so the failure is
        // actionable without re-running the gate by hand.
        let mut diff = String::new();
        for (i, (a, b)) in baseline_n.lines().zip(tree_n.lines()).enumerate() {
            if a != b {
                diff.push_str(&format!("L{i:>4}: - {a}\n        + {b}\n"));
            }
        }
        let blen = baseline_n.lines().count();
        let tlen = tree_n.lines().count();
        if blen != tlen {
            diff.push_str(&format!("(length mismatch: baseline={blen} tree={tlen})\n"));
        }
        panic!(
            "starter-flow-spi dep tree drift from baseline\n\
             baseline: {}\nfirst diffs:\n{diff}",
            baseline_path.display()
        );
    }
}

#[test]
fn no_flow_crate_depends_on_phase3_surfaces() {
    // Phase 3 (persistence + surface wrappers) is the next job:
    // wiring runs surface -> flow, never flow -> surface. If this
    // test ever fails the dependency direction has been inverted.
    let forbidden = ["starter-mcp", "starter-server", "starter-cli"];
    for pkg in [
        "starter-flow",
        "starter-flow-nodes",
        "starter-flow-spi",
        "starter-flow-surfaces",
    ] {
        let tree = cargo_tree(pkg);
        for bad in forbidden {
            // Match the bare crate name as it appears at the start
            // of a `cargo tree` node line (after the tree glyphs):
            // ` <glyphs> <name> v<ver>`. A simple substring search
            // would false-positive on e.g. workspace-root path
            // mentions; require the crate-name token followed by
            // a space and `v`.
            let needle = format!("{bad} v");
            assert!(
                !tree.contains(&needle),
                "{pkg} must not depend on Phase 3 surface crate `{bad}`:\n{tree}"
            );
        }
    }
}
