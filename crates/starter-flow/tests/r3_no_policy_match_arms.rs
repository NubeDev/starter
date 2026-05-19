//! R3 grep-contract test.
//!
//! Executable form of [DOCS/flow/scope/SCOPE.md] R3 ("the engine is
//! a reader of policies, never an owner"). This test walks every
//! `.rs` file under `crates/starter-flow/src/`, strips line/block
//! comments and (where it matters) string literals, then asserts
//! that no code path performs a literal `match` arm — or a string
//! equality compare — on any of the seven policy slot names from
//! the SCOPE R3 list:
//!
//! ```text
//!     session_policy
//!     on_failure
//!     cost_cap
//!     safe_state
//!     trigger
//!     auth
//!     timeout
//! ```
//!
//! Hits that are NOT considered violations:
//!
//! - identifiers used outside a match arm (e.g. `tokio::time::timeout`,
//!   the `triggers` field on `Topology`, the `safe_state` trait
//!   method name on `WritableOutput`, doc-comment prose, span names).
//! - the keyword appearing in `///`, `//!`, `//` or `/* … */` comments.
//! - the keyword appearing in non-arm string literals (e.g.
//!   `KindId::new("…")`, error messages, `tracing` span field values).
//!
//! Hits that ARE violations:
//!
//! - a bare identifier followed by `=>` (an arm pattern), e.g.
//!   `safe_state =>` or `timeout =>`.
//! - a string-literal arm: `"safe_state" => …`.
//! - a string-equality compare against one of the keywords:
//!   `x == "safe_state"` or `"safe_state" == x` (which would be the
//!   non-match-arm equivalent of switching on the policy name).
//!
//! The single legitimate identifier hit anywhere in the crate is the
//! `safe_state` *method name* on the `WritableOutput` trait in
//! `engine.rs` — that's the R12 hook the engine calls during graceful
//! stop, not a `match safe_state {}` switch. It is captured by the
//! method-definition (`fn safe_state`) and method-call
//! (`self.safe_state()`) sites; neither matches the violation patterns
//! above, so no explicit allow-list entry is required. This comment
//! is the justification for not adding one.
//!
//! Revisit trigger: the engine legitimately needs to dispatch on a
//! policy *slot* name (not a policy *value*) — at which point R3
//! itself is revisited, not this test.

use std::fs;
use std::path::{Path, PathBuf};

/// The seven policy slot names whose use as a match-arm pattern would
/// turn the engine into a switch over policy names.
const POLICY_SLOT_NAMES: &[&str] = &[
    "session_policy",
    "on_failure",
    "cost_cap",
    "safe_state",
    "trigger",
    "auth",
    "timeout",
];

/// Strip `//`-line comments and `/* … */` block comments (nesting
/// supported — Rust 1.0+ stable allows nested block comments). String
/// and char literals are preserved verbatim so the caller can still
/// scan for `"safe_state" =>` style violations.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut block_depth: u32 = 0;
    while i < bytes.len() {
        if block_depth > 0 {
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                block_depth += 1;
                i += 2;
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block_depth -= 1;
                i += 2;
                continue;
            }
            if bytes[i] == b'\n' {
                out.push('\n');
            }
            i += 1;
            continue;
        }
        // Line comment.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment open.
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block_depth = 1;
            i += 2;
            continue;
        }
        // String literal — copy through verbatim, honouring escapes.
        if bytes[i] == b'"' {
            out.push('"');
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                out.push(c as char);
                i += 1;
                if c == b'\\' && i < bytes.len() {
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if c == b'"' {
                    break;
                }
            }
            continue;
        }
        // Char literal — same shape, single quotes. We have to be a
        // bit careful: `'a` is also a lifetime, not a char literal.
        // We treat `'` followed by `\` or by `<char>'` as a char
        // literal; otherwise we copy the single quote and move on.
        if bytes[i] == b'\'' {
            // Look ahead for the closing single quote within 4 bytes.
            let is_char_lit = (i + 1 < bytes.len() && bytes[i + 1] == b'\\')
                || (i + 2 < bytes.len() && bytes[i + 2] == b'\'');
            if is_char_lit {
                out.push('\'');
                i += 1;
                while i < bytes.len() {
                    let c = bytes[i];
                    out.push(c as char);
                    i += 1;
                    if c == b'\\' && i < bytes.len() {
                        out.push(bytes[i] as char);
                        i += 1;
                        continue;
                    }
                    if c == b'\'' {
                        break;
                    }
                }
                continue;
            }
            // Lifetime — fall through to default copy.
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// Find every occurrence of `needle` in `hay` as a whole word
/// (Rust identifier boundary).
fn find_word(hay: &str, needle: &str) -> Vec<usize> {
    let hb = hay.as_bytes();
    let nb = needle.as_bytes();
    let mut hits = Vec::new();
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if &hb[i..i + nb.len()] == nb {
            let lhs_ok = i == 0 || !is_ident_byte(hb[i - 1]);
            let rhs_ok = i + nb.len() == hb.len() || !is_ident_byte(hb[i + nb.len()]);
            if lhs_ok && rhs_ok {
                hits.push(i);
                i += nb.len();
                continue;
            }
        }
        i += 1;
    }
    hits
}

/// True if, starting at byte offset `pos` in `code`, the next
/// non-whitespace characters are `=>`.
fn followed_by_fat_arrow(code: &str, pos: usize) -> bool {
    let b = code.as_bytes();
    let mut i = pos;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
        i += 1;
    }
    i + 1 < b.len() && b[i] == b'=' && b[i + 1] == b'>'
}

/// True if the bytes immediately before `pos` (skipping whitespace)
/// are `==`.
fn preceded_by_double_eq(code: &str, pos: usize) -> bool {
    let b = code.as_bytes();
    if pos == 0 {
        return false;
    }
    let mut i = pos;
    while i > 0 {
        let c = b[i - 1];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
            i -= 1;
            continue;
        }
        break;
    }
    i >= 2 && b[i - 1] == b'=' && b[i - 2] == b'='
}

/// True if, starting at byte offset `pos`, the next non-whitespace
/// characters are `==`.
fn followed_by_double_eq(code: &str, pos: usize) -> bool {
    let b = code.as_bytes();
    let mut i = pos;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
        i += 1;
    }
    i + 1 < b.len() && b[i] == b'=' && b[i + 1] == b'='
}

/// `1`-indexed line number for the given byte offset.
fn line_of(src: &str, pos: usize) -> usize {
    src[..pos.min(src.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

struct Violation {
    file: PathBuf,
    line: usize,
    kind: &'static str,
    keyword: &'static str,
}

fn scan_file(path: &Path) -> Vec<Violation> {
    let src = fs::read_to_string(path).expect("read .rs source");
    let code = strip_comments(&src);
    let mut violations = Vec::new();

    for &kw in POLICY_SLOT_NAMES {
        // (a) Bare-identifier match-arm pattern: `<kw> =>`.
        for pos in find_word(&code, kw) {
            let after = pos + kw.len();
            if followed_by_fat_arrow(&code, after) {
                violations.push(Violation {
                    file: path.to_path_buf(),
                    line: line_of(&code, pos),
                    kind: "ident-match-arm",
                    keyword: kw,
                });
            }
        }

        // (b/c) String-literal match-arm `"<kw>" =>` and string
        // equality `x == "<kw>"` / `"<kw>" == x`. We scan for the
        // exact byte sequence `"<kw>"` in `code` (comments already
        // stripped, strings preserved verbatim).
        let needle = format!("\"{kw}\"");
        let mut start = 0;
        while let Some(rel) = code[start..].find(&needle) {
            let pos = start + rel;
            let after = pos + needle.len();
            if followed_by_fat_arrow(&code, after) {
                violations.push(Violation {
                    file: path.to_path_buf(),
                    line: line_of(&code, pos),
                    kind: "string-match-arm",
                    keyword: kw,
                });
            }
            if preceded_by_double_eq(&code, pos) || followed_by_double_eq(&code, after) {
                violations.push(Violation {
                    file: path.to_path_buf(),
                    line: line_of(&code, pos),
                    kind: "string-eq-compare",
                    keyword: kw,
                });
            }
            start = after;
        }
    }

    violations
}

fn src_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("src")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
}

#[test]
fn no_policy_name_match_arms_in_engine_src() {
    let root = src_dir();
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no .rs files found under {} — test harness misconfigured",
        root.display()
    );

    let mut all = Vec::new();
    for f in &files {
        all.extend(scan_file(f));
    }

    if !all.is_empty() {
        let mut msg = String::from(
            "R3 grep-contract violation: the engine must not match or compare on \
             policy slot names (DOCS/flow/scope/SCOPE.md R3). Hits:\n",
        );
        for v in &all {
            msg.push_str(&format!(
                "  {}:{}  [{}]  keyword={}\n",
                v.file.display(),
                v.line,
                v.kind,
                v.keyword,
            ));
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------
// Self-tests for the line-oriented tokeniser. These guard against
// regressions where the scanner stops noticing a real violation or
// starts flagging a doc comment.
// ---------------------------------------------------------------

#[test]
fn detects_identifier_match_arm() {
    // Synthetic violation: a `match` arm whose pattern is `timeout`.
    // We feed the scanner a small in-memory blob rather than a file
    // so the test is hermetic.
    let src = r#"
fn switch(p: P) {
    match p {
        timeout => 1,
        _ => 0,
    };
}
"#;
    let code = strip_comments(src);
    let hits = find_word(&code, "timeout");
    assert_eq!(hits.len(), 1);
    assert!(followed_by_fat_arrow(&code, hits[0] + "timeout".len()));
}

#[test]
fn ignores_keyword_in_line_comment() {
    let src = "// safe_state => 1\nlet x = 1;\n";
    let code = strip_comments(src);
    assert!(!code.contains("safe_state"));
}

#[test]
fn ignores_keyword_in_doc_comment() {
    let src = "/// the safe_state hook drives writable outputs\nfn f() {}\n";
    let code = strip_comments(src);
    assert!(!code.contains("safe_state"));
}

#[test]
fn ignores_keyword_in_block_comment() {
    let src = "/* timeout => trap */ fn f() {}\n";
    let code = strip_comments(src);
    assert!(!code.contains("timeout"));
}

#[test]
fn ignores_keyword_as_function_call() {
    // `tokio::time::timeout(…)` is fine — not a match arm.
    let src = "let r = tokio::time::timeout(d, fut).await;\n";
    let code = strip_comments(src);
    let hits = find_word(&code, "timeout");
    assert_eq!(hits.len(), 1);
    assert!(!followed_by_fat_arrow(&code, hits[0] + "timeout".len()));
}

#[test]
fn detects_string_match_arm() {
    let src = r#"
fn switch(s: &str) {
    match s {
        "safe_state" => 1,
        _ => 0,
    };
}
"#;
    let code = strip_comments(src);
    let needle = "\"safe_state\"";
    let pos = code.find(needle).expect("string literal present");
    assert!(followed_by_fat_arrow(&code, pos + needle.len()));
}

#[test]
fn detects_string_equality_compare() {
    let src = "if name == \"on_failure\" { 1 } else { 0 };\n";
    let code = strip_comments(src);
    let needle = "\"on_failure\"";
    let pos = code.find(needle).expect("string literal present");
    assert!(preceded_by_double_eq(&code, pos));
}

#[test]
fn ignores_non_arm_string_literal() {
    // `KindId::new("trigger")` — string is data, not a match arm or
    // an equality compare.
    let src = "let k = KindId::new(\"trigger\").unwrap();\n";
    let code = strip_comments(src);
    let needle = "\"trigger\"";
    let pos = code.find(needle).expect("string literal present");
    assert!(!followed_by_fat_arrow(&code, pos + needle.len()));
    assert!(!preceded_by_double_eq(&code, pos));
    assert!(!followed_by_double_eq(&code, pos + needle.len()));
}
