//! SCOPE smoke 5 — **R8**.
//!
//! Every public item in `starter-spi`'s blob additions has a
//! doc-comment that explains *why* the shape is what it is.
//! Stage 1 wrote the rustdoc; this smoke locks it down so a
//! later refactor cannot quietly strip a doc-comment off a
//! public item without the test going red.
//!
//! The check is a lightweight syntactic scan of
//! `crates/starter-spi/src/blob/**.rs`: for every line that
//! declares a `pub` item (`pub fn`, `pub struct`, `pub enum`,
//! `pub trait`, `pub const`, `pub type`, `pub use` is exempt
//! because it just re-exports), the immediately preceding
//! non-empty, non-attribute line must be a `///` doc-comment.
//! Pure tokenisation, no syntax tree — we want this readable in a
//! single screen and not depend on `syn` at workspace test time.
//!
//! The "why" half of R8 is reviewed at PR time (a one-liner like
//! `/// the size in bytes` would pass this smoke but fail review);
//! the syntactic floor below catches the regression where the
//! whole doc-comment vanishes.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root above crates/smoke-tests")
        .to_path_buf()
}

fn is_pub_item_line(trimmed: &str) -> bool {
    // We deliberately exclude `pub use` (re-exports) and
    // `pub(crate)` / `pub(super)` (not part of the public API).
    if !trimmed.starts_with("pub ") {
        return false;
    }
    if trimmed.starts_with("pub use ") || trimmed.starts_with("pub(") {
        return false;
    }
    // Match the kinds of items the SCOPE's R8 wants documented.
    [
        "pub fn ",
        "pub async fn ",
        "pub unsafe fn ",
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub const ",
        "pub static ",
        "pub type ",
        "pub mod ",
    ]
    .iter()
    .any(|kw| trimmed.starts_with(kw))
}

fn check_file(path: &Path) -> Vec<String> {
    let src = fs::read_to_string(path).expect("read blob spi file");
    let lines: Vec<&str> = src.lines().collect();
    let mut violations = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !is_pub_item_line(trimmed) {
            continue;
        }
        // Walk back over attributes and empty lines.
        let mut probe = idx;
        let had_doc = loop {
            if probe == 0 {
                break false;
            }
            probe -= 1;
            let prev = lines[probe].trim_start();
            if prev.is_empty() {
                continue;
            }
            if prev.starts_with("#[") || prev.starts_with("#![") {
                continue;
            }
            break prev.starts_with("///") || prev.starts_with("//!");
        };
        if !had_doc {
            violations.push(format!("{}:{}: `{}`", path.display(), idx + 1, trimmed));
        }
    }
    violations
}

#[test]
fn every_public_blob_item_has_a_doc_comment() {
    let blob_dir = repo_root().join("crates/starter-spi/src/blob");
    let mut violations = Vec::new();
    for entry in fs::read_dir(&blob_dir).expect("read blob dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            violations.extend(check_file(&path));
        }
    }
    assert!(
        violations.is_empty(),
        "R8 violation — public blob items without doc-comments:\n{}",
        violations.join("\n"),
    );
}
