//! PR 5 — CLI parity for the `system disk` verb.
//!
//! Drives the `rubix-admin` binary built by Cargo for tests
//! (`CARGO_BIN_EXE_rubix-admin`), asserting:
//!
//! 1. `LANG=en_US.UTF-8 rubix-admin system disk` produces sensible
//!    English output (EN catalogue rendering reached via
//!    `MessageBundle::render_diagnostic`).
//! 2. `LANG=es_AR.UTF-8 rubix-admin system disk` produces Spanish
//!    output (ES catalogue rendering reached via the same renderer).
//! 3. `rubix-admin system disk --json` skips the render and dumps a
//!    JSON object containing `summary.code` and `summary.params`.
//! 4. The CLI does not open a TCP connection back to itself — the
//!    rubix-side source contains no `reqwest` / `hyper::Client` /
//!    `TcpStream` reference (grep guard).

use std::path::PathBuf;
use std::process::Command;

fn cli_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rubix-admin"))
}

fn run_with(lang: &str, extra: &[&str]) -> (String, String, i32) {
    let mut cmd = Command::new(cli_path());
    cmd.env("LANG", lang)
        .env_remove("LC_ALL")
        .arg("system")
        .arg("disk");
    for a in extra {
        cmd.arg(a);
    }
    let out = cmd.output().expect("spawn rubix-admin");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn english_locale_renders_english() {
    let (stdout, stderr, code) = run_with("en_US.UTF-8", &[]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        ["Disk usage is normal", "Disk is nearly full", "Disk is full"]
            .iter()
            .any(|p| stdout.contains(p)),
        "EN catalogue not reached: stdout={stdout}",
    );
}

#[test]
fn spanish_locale_renders_spanish() {
    let (stdout, stderr, code) = run_with("es_AR.UTF-8", &[]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    assert!(
        [
            "El uso del disco es normal",
            "El disco está casi lleno",
            "El disco está lleno",
        ]
        .iter()
        .any(|p| stdout.contains(p)),
        "ES catalogue not reached: stdout={stdout}",
    );
}

#[test]
fn json_flag_dumps_summary_code_and_params() {
    let (stdout, stderr, code) = run_with("en_US.UTF-8", &["--json"]);
    assert_eq!(code, 0, "stdout={stdout} stderr={stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"));
    let summary = parsed
        .get("summary")
        .unwrap_or_else(|| panic!("missing summary: {parsed}"));
    let code = summary
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("summary.code missing: {summary}"));
    assert!(
        code.starts_with("rubix.system.disk."),
        "summary.code must be a rubix.system.disk.* key; got {code:?}",
    );
    assert!(
        summary.get("params").is_some(),
        "summary.params must be present (even if empty): {summary}",
    );
}

#[test]
fn cli_source_does_not_open_tcp_to_itself() {
    // The CLI is an in-process consumer of `probe()`. If a future
    // edit pulls in `reqwest`, `hyper::Client`, or `TcpStream` on
    // the rubix-admin side, this test fails — push the change back
    // into the shared `probe()` rather than introducing a self-call.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cli_root = here.join("src/bin/rubix_admin");
    let banned = ["reqwest", "hyper::Client", "TcpStream"];
    walk(&cli_root, &mut |path, content| {
        for needle in banned {
            assert!(
                !content.contains(needle),
                "{} must not reference {needle}",
                path.display(),
            );
        }
    });
}

fn walk(dir: &std::path::Path, visit: &mut dyn FnMut(&std::path::Path, &str)) {
    for entry in std::fs::read_dir(dir).expect("readdir") {
        let entry = entry.expect("dirent");
        let path = entry.path();
        if path.is_dir() {
            walk(&path, visit);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let content = std::fs::read_to_string(&path).expect("read");
            visit(&path, &content);
        }
    }
}
