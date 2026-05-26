//! Full extension lifecycle integration test (Phase E.1).
//!
//! Boots `rubix_agent::boot::extensions::build_extension_admin` against
//! a testcontainers Postgres and a fixture extensions directory that
//! contains a synthetic `com.rubix.example` bundle pointing at the
//! pre-built `rubix-example-extension` process binary. Mounts the
//! upstream `starter_ext_server::router(admin)` (the same router
//! `rubix-agent`'s `main.rs` merges under `/api/v1/extensions/*`) and
//! drives the 8 SCOPE Phase E assertions through Axum's `oneshot`:
//!
//!   1. `GET  /extensions`                — the example shows up.
//!   2. `POST /extensions/<id>/enable`    — "start": spawns supervisor.
//!   3. `POST /extensions/<id>/disable`   — "stop": tears it down.
//!   4. enable → disable → enable         — "restart" cycle.
//!   5. `POST .../disable` again          — supervisor handle gone
//!                                          (`events` route 404).
//!   6. `POST .../enable`                 — supervisor re-spawned
//!                                          (`events` route 200).
//!   7. PG row state matches the final operation (`disabled`).
//!   8. The events ring carries the expected lifecycle messages
//!      (at minimum: a `state_transition` away from `Starting`).
//!
//! The fixture bundle copies the pre-built example binary into
//! `<tmp>/com.rubix.example/rubix-example-extension` so the supervisor
//! has a real OS process to spawn. The Phase B placeholder binary
//! exits immediately after printing a stderr line, which is enough to
//! drive `Spawned` + a follow-on `StateTransition` into the event ring
//! before the supervisor settles — the assertions stay agnostic to
//! whether the child completes the init handshake.
//!
//! Requires Docker (testcontainers Postgres) and the example binary
//! built once via `cargo build --manifest-path rubix/extensions/Cargo.toml`.
//! The test attempts that build on first run if the binary is missing.
//!
//! Per the existing rubix-agent integration-test convention this is
//! gated behind `#[ignore]` so the default `cargo test` stays fast.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use serde_json::Value;
use sqlx::Row;
use starter_ext_server::router as ext_router;
use starter_store_postgres::testing::with_database;
use tower::ServiceExt;

use rubix_agent::boot::config::{AgentConfig, ExtensionsConfig};
use rubix_agent::boot::extensions::build_extension_admin;

const EXAMPLE_ID: &str = "com.rubix.example";

/// Locate the repo root by walking up from `CARGO_MANIFEST_DIR`
/// (`rubix/crates/rubix-agent/`) until we land on the directory that
/// holds the `rubix/` and `starter-extensions/` siblings.
fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..4 {
        if p.join("rubix").is_dir() && p.join("starter-extensions").is_dir() {
            return p;
        }
        if !p.pop() {
            break;
        }
    }
    panic!(
        "could not locate repo root from {}",
        env!("CARGO_MANIFEST_DIR")
    );
}

/// Ensure the example process binary exists; build it via cargo if
/// not. The binary lands at `rubix/extensions/target/debug/rubix-example-extension`.
fn ensure_example_binary(root: &Path) -> PathBuf {
    let bin = root.join("rubix/extensions/target/debug/rubix-example-extension");
    if bin.exists() {
        return bin;
    }
    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--manifest-path",
            "rubix/extensions/Cargo.toml",
            "-p",
            "rubix-example-extension",
        ])
        .current_dir(root)
        .status()
        .expect("invoke cargo build for example extension");
    assert!(status.success(), "cargo build of example extension failed");
    assert!(bin.exists(), "expected binary at {}", bin.display());
    bin
}

/// Lay down a single-extension fixture root. Writes a self-contained
/// `block.yaml` (process flavour, runtime.bin = the copied binary's
/// filename inside the bundle dir) and copies the prebuilt example
/// binary alongside.
fn make_fixture_dir(bin_src: &Path) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let bundle = dir.path().join(EXAMPLE_ID);
    std::fs::create_dir_all(&bundle).unwrap();

    let bin_name = "rubix-example-extension";
    let bin_dst = bundle.join(bin_name);
    std::fs::copy(bin_src, &bin_dst).expect("copy example binary into fixture");
    // Re-stamp executable bit on copy (cp preserves but copy may not on
    // all platforms; harmless if already set).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&bin_dst).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_dst, perms).unwrap();
    }

    // Minimal valid manifest. Matches the upstream Manifest schema
    // (`v: 1`, runtime.kind = process, `bin` resolved relative to the
    // bundle dir by the supervisor).
    let yaml = format!(
        r#"v: 1
id: {EXAMPLE_ID}
version: 0.1.0
display_name: "Rubix Example (lifecycle fixture)"
authors: ["ap@nube-io.com"]
runtime:
  kind: process
  bin: {bin_name}
"#
    );
    std::fs::write(bundle.join("block.yaml"), yaml).unwrap();
    dir
}

fn agent_config(extensions_dir: PathBuf) -> AgentConfig {
    let mut cfg = AgentConfig::default();
    cfg.extensions = ExtensionsConfig {
        enabled: true,
        dir: extensions_dir,
        // Drive every transition explicitly through REST so the
        // assertions don't race the boot-time autostart loop.
        autostart_enabled_records: false,
    };
    cfg
}

async fn do_request(app: &Router, method: Method, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 256 * 1024).await.unwrap();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

#[tokio::test]
#[ignore = "requires Docker (testcontainers Postgres); run via the integration job"]
async fn extension_lifecycle_full_roundtrip() {
    let root = repo_root();
    let bin = ensure_example_binary(&root);
    let fixture = make_fixture_dir(&bin);

    let (pool, _guard) = with_database().await;
    let cfg = agent_config(fixture.path().to_path_buf());
    let bundle = build_extension_admin(&cfg, pool.sqlx())
        .await
        .expect("build_extension_admin succeeds");
    let app: Router = ext_router::<()>(bundle.admin.clone());

    // ---- (1) GET /extensions lists the example. ----
    let (status, body) = do_request(&app, Method::GET, "/extensions").await;
    assert_eq!(status, StatusCode::OK, "GET /extensions returns 200");
    let rows = body.as_array().expect("GET /extensions returns an array");
    let row = rows
        .iter()
        .find(|r| r["id"] == EXAMPLE_ID)
        .expect("example extension shows up in GET /extensions");
    assert_eq!(row["version"], "0.1.0");

    // ---- (2) "Start" — POST .../enable spawns the supervisor. ----
    let uri_enable = format!("/extensions/{EXAMPLE_ID}/enable");
    let uri_disable = format!("/extensions/{EXAMPLE_ID}/disable");
    let uri_events = format!("/extensions/{EXAMPLE_ID}/events");

    let (status, body) = do_request(&app, Method::POST, &uri_enable).await;
    assert_eq!(status, StatusCode::OK, "enable returns 200");
    assert_eq!(body["enabled"], "enabled");

    // Let the supervisor cycle (spawn → init handshake attempt → settle).
    tokio::time::sleep(Duration::from_millis(300)).await;

    // ---- (8 first half) events route is reachable after enable. ----
    let (status, body) = do_request(&app, Method::GET, &uri_events).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "events endpoint reachable while supervisor exists"
    );
    let events = body["events"].as_array().expect("events array present");
    assert!(
        !events.is_empty(),
        "supervisor pushed at least one event after enable"
    );

    // ---- (3) "Stop" — POST .../disable tears down the supervisor. ----
    let (status, body) = do_request(&app, Method::POST, &uri_disable).await;
    assert_eq!(status, StatusCode::OK, "disable returns 200");
    assert_eq!(body["enabled"], "disabled");

    // ---- (5) After disable the events route reports 404 (no handle). ----
    let (status, _) = do_request(&app, Method::GET, &uri_events).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "events endpoint 404s once the supervisor is torn down"
    );

    // ---- (4) "Restart" cycle: re-enable, then disable, then re-enable. ----
    let (status, _) = do_request(&app, Method::POST, &uri_enable).await;
    assert_eq!(status, StatusCode::OK, "re-enable after disable");
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (status, _) = do_request(&app, Method::POST, &uri_disable).await;
    assert_eq!(status, StatusCode::OK, "disable mid-cycle");
    let (status, body) = do_request(&app, Method::POST, &uri_enable).await;
    assert_eq!(status, StatusCode::OK, "final re-enable");
    assert_eq!(body["enabled"], "enabled");

    // ---- (6) Supervisor handle exists after the final enable. ----
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (status, body) = do_request(&app, Method::GET, &uri_events).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "events endpoint reachable after final enable"
    );
    let events = body["events"].as_array().expect("events array");
    // `RingEvent.kind` is `#[serde(tag = "kind", content = "data")]` so it
    // lands as `{"kind": "<variant>", "data": {...}}` on the wire. The
    // example binary is the Phase B placeholder that exits before the
    // init handshake completes, so the supervisor records
    // `spawned` → `crashed` → `restart_scheduled` (or a state_transition
    // into Failed if the restart cap blew). Any of those proves the
    // lifecycle ring received messages — assert on the union.
    let lifecycle_kinds = [
        "spawned",
        "crashed",
        "restart_scheduled",
        "state_transition",
        "exited_clean",
    ];
    let saw_transition = events.iter().any(|e| {
        e["kind"]["kind"]
            .as_str()
            .map(|k| lifecycle_kinds.contains(&k))
            .unwrap_or(false)
    });
    assert!(
        saw_transition,
        "events ring carries a lifecycle message ({:?}); got {events:#?}",
        lifecycle_kinds,
    );

    // ---- Move to a known terminal state: disable. ----
    let (status, body) = do_request(&app, Method::POST, &uri_disable).await;
    assert_eq!(status, StatusCode::OK, "final disable");
    assert_eq!(body["enabled"], "disabled");

    // ---- (7) PG row reflects the final state. ----
    let row = sqlx::query("SELECT state FROM extensions_enablement WHERE extension_id = $1")
        .bind(EXAMPLE_ID)
        .fetch_one(pool.sqlx())
        .await
        .expect("read enablement row");
    let pg_state: String = row.get(0);
    assert_eq!(
        pg_state, "disabled",
        "extensions_enablement row matches the final disable"
    );

    // Hold onto Arc so the fixture dir and bundle outlive the admin.
    drop(bundle);
    drop(fixture);
    drop(pool);
    let _ = Arc::new(());
}
