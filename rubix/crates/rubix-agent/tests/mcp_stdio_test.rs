//! Integration test for the `rubix-admin mcp` stdio transport.
//!
//! Spawns the `rubix-admin` binary as a child process, writes
//! Content-Length-framed JSON-RPC frames to its stdin, and reads the
//! framed responses back from stdout. The bundled flows are now
//! driven through a real [`starter_ai_agent::AgentLoop`] so the
//! tests use a recorded-LLM fixture under `tests/fixtures/` —
//! `RUBIX_AI_FIXTURE` swaps the default [`ClaudeRunner`] for a JSON-
//! script replay runner. **No live LLM is hit in CI.**
//!
//! Block C deleted the hand-rolled `com.rubix.diag-render` node, so
//! the previous exact-string assertions on Spanish disk-rendering
//! output are gone. The structural assertions that remain are:
//!
//!   1. `initialize` succeeds.
//!   2. `tools/list` lists all six bundled flows — at minimum the
//!      `com.rubix.scheduled-system-check` entry.
//!   3. `tools/call` against that flow round-trips a non-error
//!      response whose payload carries `code` matching the
//!      `rubix.system.disk.*` shape and a numeric `params.percent`.
//!   4. Same call in both `en-US` and `es-AR` locales (the
//!      acceptLanguage cascade still routes — the wording is
//!      LLM-supplied via the fixture).

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

/// Path under `target/debug/` for the binary `cargo test` built.
fn binary_path() -> std::path::PathBuf {
    // `CARGO_BIN_EXE_<name>` is set by cargo for integration tests
    // and points at the freshly-built binary. The name comes from
    // the `[[bin]] name = "rubix-admin"` entry in Cargo.toml.
    let path = env!("CARGO_BIN_EXE_rubix-admin");
    std::path::PathBuf::from(path)
}

/// Build a Content-Length-framed JSON-RPC frame the
/// `starter-jsonrpc-stdio` reader will accept.
fn frame(value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).expect("serialise JSON-RPC frame");
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(&body);
    out
}

/// Read one Content-Length-framed JSON-RPC frame from `reader`.
async fn read_frame<R: tokio::io::AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Value {
    use tokio::io::AsyncBufReadExt;
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = tokio::time::timeout(Duration::from_secs(30), reader.read_line(&mut line))
            .await
            .expect("read header line: timeout")
            .expect("read header line");
        if n == 0 {
            panic!("stdout EOF while reading frame headers");
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                rest.trim()
                    .parse()
                    .expect("Content-Length header parses as usize"),
            );
        }
    }
    let len = content_length.expect("Content-Length header present");
    let mut body = vec![0u8; len];
    tokio::time::timeout(Duration::from_secs(30), reader.read_exact(&mut body))
        .await
        .expect("read body: timeout")
        .expect("read body");
    serde_json::from_slice(&body).expect("response body parses as JSON")
}

fn fixture_path(name: &str) -> String {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures");
    p.push(name);
    p.to_string_lossy().into_owned()
}

async fn spawn_admin(principal_email: &str, fixture: Option<&str>) -> Child {
    let mut cmd = Command::new(binary_path());
    cmd.arg("mcp")
        .env_remove("RUBIX_DATABASE_URL")
        .env_remove("RUBIX_CONFIG")
        .env("RUBIX_PRINCIPAL_EMAIL", principal_email)
        .env("LANG", "en_US.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match fixture {
        Some(path) => {
            cmd.env("RUBIX_AI_FIXTURE", path);
        }
        None => {
            cmd.env_remove("RUBIX_AI_FIXTURE");
        }
    }
    cmd.spawn().expect("spawn rubix-admin mcp")
}

async fn drive_call(accept_language: &str, fixture_file: &str) -> Value {
    let fixture = fixture_path(fixture_file);
    let mut child = spawn_admin("op@example.com", Some(&fixture)).await;
    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let init = frame(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "_meta": { "acceptLanguage": accept_language } }
    }));
    stdin.write_all(&init).await.expect("write initialize");
    let _init_resp = read_frame(&mut reader).await;

    let list = frame(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));
    stdin.write_all(&list).await.expect("write tools/list");
    let list_resp = read_frame(&mut reader).await;
    let names: Vec<String> = list_resp["result"]["tools"]
        .as_array()
        .map(|tools| {
            tools
                .iter()
                .filter_map(|t| t["name"].as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        names.iter().any(|n| n == "com.rubix.scheduled-system-check"),
        "tools/list must surface com.rubix.scheduled-system-check; saw {names:?}",
    );
    // After Block A the bundled-flow loader registers all six rubix
    // flows; assert we have at least that many.
    assert!(
        names.len() >= 6,
        "tools/list must surface every bundled flow (>=6); saw {} -> {names:?}",
        names.len(),
    );

    let call = frame(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "com.rubix.scheduled-system-check",
            "arguments": {}
        }
    }));
    stdin.write_all(&call).await.expect("write tools/call");
    let call_resp = read_frame(&mut reader).await;
    assert!(
        call_resp["error"].is_null(),
        "tools/call must succeed; got {call_resp}",
    );

    // Close stdin so the loop exits, then reap.
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
    call_resp
}

/// Structural assertion (`code` + numeric `params.percent`/`params.free`)
/// against the JSON payload the rubix `ai-agent` node writes to its
/// `out` slot. The exact wording is LLM-supplied (fixture-driven);
/// only the shape is invariant.
fn assert_disk_shape(call_resp: &Value) {
    let payload = &call_resp["result"]["structuredContent"];
    let code = payload["code"]
        .as_str()
        .unwrap_or_else(|| panic!("expected `code` string in {payload}"));
    assert!(
        code.starts_with("rubix.system.disk."),
        "code must be rubix.system.disk.{{ok,warn,full}}; got {code:?}",
    );
    assert!(
        payload["params"]["percent"].is_number(),
        "params.percent must be numeric; got {payload}",
    );
    assert!(
        payload["params"]["free"].is_number(),
        "params.free must be numeric; got {payload}",
    );
    // The model's final reply text is present (whatever the fixture supplied).
    assert!(
        payload["reply"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "reply text must be non-empty; got {payload}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_round_trips_in_en_us() {
    let resp = drive_call("en-US", "scheduled-system-check-en.json").await;
    assert_disk_shape(&resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_round_trips_in_es_ar() {
    let resp = drive_call("es-AR", "scheduled-system-check-es.json").await;
    assert_disk_shape(&resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_exits_nonzero_when_principal_email_missing() {
    let mut cmd = Command::new(binary_path());
    cmd.arg("mcp")
        .env_remove("RUBIX_PRINCIPAL_EMAIL")
        .env_remove("RUBIX_DATABASE_URL")
        .env_remove("RUBIX_CONFIG")
        .env_remove("RUBIX_AI_FIXTURE")
        .env("LANG", "es_AR.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn rubix-admin mcp");
    let status = tokio::time::timeout(Duration::from_secs(15), child.wait())
        .await
        .expect("missing-email path exits within 15s")
        .expect("wait status");
    assert!(
        !status.success(),
        "missing RUBIX_PRINCIPAL_EMAIL must exit non-zero; got {status:?}",
    );
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr")
        .read_to_string(&mut stderr)
        .await
        .expect("read stderr");
    assert!(
        stderr.contains("RUBIX_PRINCIPAL_EMAIL no está definido"),
        "missing-principal stderr must carry the localised Spanish diagnostic; got {stderr:?}",
    );
}

/// DB-backed assertion: an unknown principal exits non-zero with a
/// localised not-found diagnostic. Requires Postgres — `#[ignore]`d.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a testcontainers Postgres with the bootstrap_user fixture"]
async fn mcp_stdio_exits_nonzero_for_unknown_principal_email() {
    let dsn = std::env::var("RUBIX_TEST_DSN")
        .expect("set RUBIX_TEST_DSN to a live Postgres before un-ignoring");
    let mut cmd = Command::new(binary_path());
    cmd.arg("mcp")
        .env("RUBIX_PRINCIPAL_EMAIL", "missing@example.com")
        .env("RUBIX_DATABASE_URL", dsn)
        .env_remove("RUBIX_CONFIG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn rubix-admin mcp");
    let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
        .await
        .expect("unknown-principal path exits within 30s")
        .expect("wait status");
    assert!(!status.success());
}
