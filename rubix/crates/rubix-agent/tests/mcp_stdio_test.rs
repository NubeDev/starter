//! Integration test for the `rubix-admin mcp` stdio transport.
//!
//! Spawns the `rubix-admin` binary as a child process, writes
//! Content-Length-framed JSON-RPC frames to its stdin, and reads
//! the framed responses back from stdout. The session walks the
//! same sequence Claude Desktop performs:
//!
//!   1. `initialize` with `params._meta.acceptLanguage` set.
//!   2. `tools/list` — must surface
//!      `com.rubix.scheduled-system-check`.
//!   3. `tools/call` against the same tool — must render the
//!      bundled diagnostic in the requested locale, with the
//!      matching timezone shift and date pattern.
//!
//! The default-enabled tests exercise the path with
//! `RUBIX_DATABASE_URL` unset so the binary takes its no-DSN
//! tolerance branch (matches the rubix-agent HTTP binary's
//! behaviour without a database). The DB-backed path uses a
//! testcontainers-style Postgres fixture and is `#[ignore]`d so it
//! does not gate CI; the same gating is already applied to
//! `tests/authz_gate_test.rs`.

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
async fn read_frame<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> Value {
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

async fn spawn_admin(principal_email: &str) -> Child {
    let mut cmd = Command::new(binary_path());
    cmd.arg("mcp")
        // Critical: do NOT inherit RUBIX_DATABASE_URL from the
        // host shell — the default-enabled tests run without a
        // real Postgres and rely on the binary's no-DSN tolerance.
        .env_remove("RUBIX_DATABASE_URL")
        .env_remove("RUBIX_CONFIG")
        .env("RUBIX_PRINCIPAL_EMAIL", principal_email)
        .env("LANG", "en_US.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.spawn().expect("spawn rubix-admin mcp")
}

async fn drive_call(accept_language: &str) -> String {
    let mut child = spawn_admin("op@example.com").await;
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
    let rendered = call_resp["result"]["structuredContent"]["rendered"]
        .as_str()
        .unwrap_or_else(|| panic!("expected `rendered` string in {call_resp}"))
        .to_owned();

    // Close stdin so the loop exits, then reap.
    drop(stdin);
    let _ = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
    rendered
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_renders_in_en_us() {
    let rendered = drive_call("en-US").await;
    assert!(
        rendered.starts_with("Disk is nearly full"),
        "EN rendering must use English catalogue; got {rendered:?}",
    );
    assert!(
        rendered.contains("01/15/2024"),
        "EN rendering must use US date format MM/DD/YYYY; got {rendered:?}",
    );
    assert!(
        rendered.contains("07:00"),
        "EN rendering must shift into America/New_York (07:00); got {rendered:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_renders_in_es_ar() {
    let rendered = drive_call("es-AR").await;
    assert!(
        rendered.starts_with("El disco está casi lleno"),
        "ES rendering must use Spanish catalogue; got {rendered:?}",
    );
    assert!(
        rendered.contains("15/01/2024"),
        "ES rendering must use EU date format DD/MM/YYYY; got {rendered:?}",
    );
    assert!(
        rendered.contains("09:00"),
        "ES rendering must shift into America/Argentina/Buenos_Aires (09:00); got {rendered:?}",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_stdio_exits_nonzero_when_principal_email_missing() {
    let mut cmd = Command::new(binary_path());
    cmd.arg("mcp")
        .env_remove("RUBIX_PRINCIPAL_EMAIL")
        .env_remove("RUBIX_DATABASE_URL")
        .env_remove("RUBIX_CONFIG")
        .env("LANG", "es_AR.UTF-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn rubix-admin mcp");
    // No stdin frames — the binary should fail the principal
    // resolution before reading anything.
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
    // Spanish catalogue rendering of the missing-principal key.
    assert!(
        stderr.contains("RUBIX_PRINCIPAL_EMAIL no está definido"),
        "missing-principal stderr must carry the localised Spanish diagnostic; got {stderr:?}",
    );
}

/// DB-backed assertion: a `RUBIX_PRINCIPAL_EMAIL` that does not
/// map to a real user must exit non-zero with the localised
/// not_found diagnostic on stderr. Requires a live Postgres; the
/// test is `#[ignore]`d so CI doesn't need a container. Mirrors
/// the gating already applied to `authz_gate_test.rs`.
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
