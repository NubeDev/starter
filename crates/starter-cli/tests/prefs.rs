//! End-to-end tests for the `prefs` subcommand.
//!
//! Spins up a real HTTP server with the `starter-prefs` router
//! mounted against an in-memory sqlite store, then drives each
//! `prefs {get,set,units}` subcommand through `run_prefs_with`
//! (the test-only stdout-capture seam on the CLI's `Prefs`
//! command) and asserts on the captured bytes.
//!
//! The server is wrapped in a tower layer that injects a fixed
//! `Principal` extension on every request — `starter-prefs` reads
//! the principal off the request's extensions, so this gives the
//! routes the auth context they need without standing up the full
//! auth stack.

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{from_fn, Next};
use axum::response::Response;
use axum::Router;
use serde_json::json;
use sqlx::SqlitePool;
use starter_cli::commands::{run_prefs_with, Prefs};
use starter_cli::Command;
use starter_prefs::resolver::SystemDefaults;
use starter_prefs::routes::{prefs_router, PrefsRoutesState};
use starter_prefs::store::SqlitePrefsStore;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role, Scope};

// ---------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------

async fn spawn_prefs_server() -> TestApp {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let store = SqlitePrefsStore::new(pool);
    store.migrate().await.unwrap();
    let state = PrefsRoutesState::new(Arc::new(store), SystemDefaults::starter());
    let router: Router = prefs_router::<()>(state)
        .with_state(())
        .layer(from_fn(inject_principal));
    TestApp::spawn(router).await
}

async fn inject_principal(mut req: Request<Body>, next: Next) -> Response {
    req.extensions_mut().insert(Principal {
        subject: "alice".into(),
        role: Role::Admin,
        scopes: Vec::<Scope>::new(),
        tenant_id: None,
        extra: json!({ "active_workspace": "ws1" }),
    });
    next.run(req).await
}

async fn run(base_url: &str, sub: &str, extra: &[&str]) -> String {
    let mut full: Vec<String> = vec!["starter".into(), "prefs".into(), sub.into()];
    for a in extra {
        full.push((*a).into());
    }
    full.push("--base-url".into());
    full.push(base_url.into());
    let root = clap::Command::new("starter").subcommand(Prefs.subcommand());
    let matches = root.get_matches_from(full);
    let prefs_m = matches
        .subcommand_matches("prefs")
        .expect("prefs subcommand parsed");
    let mut buf: Vec<u8> = Vec::new();
    run_prefs_with(&mut buf, prefs_m).await.expect("prefs ok");
    String::from_utf8(buf).expect("utf-8 stdout")
}

// ---------------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------------

#[tokio::test]
async fn prefs_get_table_output() {
    let app = spawn_prefs_server().await;
    let out = run(&app.base_url, "get", &[]).await;
    // Default-shaped table: every ResolvedPreferences field present
    // on its own line, two-column key/value layout.
    assert!(out.contains("timezone"), "missing timezone line: {out}");
    assert!(out.contains("locale"), "missing locale line: {out}");
    assert!(out.contains("temperature_unit"), "missing temp line: {out}");
    assert!(out.contains("currency"), "missing currency line: {out}");
    app.shutdown().await;
}

#[tokio::test]
async fn prefs_get_json_output() {
    let app = spawn_prefs_server().await;
    let out = run(&app.base_url, "get", &["--output", "json"]).await;
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect("json parses");
    assert!(parsed.get("timezone").is_some());
    assert!(parsed.get("locale").is_some());
    assert!(parsed.get("temperature_unit").is_some());
    app.shutdown().await;
}

#[tokio::test]
async fn prefs_set_then_get_reflects_change() {
    let app = spawn_prefs_server().await;
    // Set temperature_unit to fahrenheit.
    let set_out = run(
        &app.base_url,
        "set",
        &["--field", "temperature_unit", "--value", "fahrenheit"],
    )
    .await;
    assert_eq!(set_out.trim(), "ok");

    // GET reflects the change.
    let out = run(&app.base_url, "get", &["--output", "json"]).await;
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(parsed["temperature_unit"], "fahrenheit");
    app.shutdown().await;
}

#[tokio::test]
async fn prefs_set_auto_reverts_to_inherit() {
    let app = spawn_prefs_server().await;
    // First override.
    run(
        &app.base_url,
        "set",
        &["--field", "temperature_unit", "--value", "fahrenheit"],
    )
    .await;
    let mid = run(&app.base_url, "get", &["--output", "json"]).await;
    let parsed: serde_json::Value = serde_json::from_str(mid.trim()).unwrap();
    assert_eq!(parsed["temperature_unit"], "fahrenheit");

    // Then `--value auto` reverts to inherit; with no org/user row
    // the system default takes over. SystemDefaults::starter uses
    // metric → celsius.
    let revert = run(
        &app.base_url,
        "set",
        &["--field", "temperature_unit", "--value", "auto"],
    )
    .await;
    assert_eq!(revert.trim(), "ok");

    let after = run(&app.base_url, "get", &["--output", "json"]).await;
    let parsed: serde_json::Value = serde_json::from_str(after.trim()).unwrap();
    assert_eq!(parsed["temperature_unit"], "celsius");
    app.shutdown().await;
}

#[tokio::test]
async fn prefs_units_lists_registry() {
    let app = spawn_prefs_server().await;
    let out = run(&app.base_url, "units", &[]).await;
    // The closed registry covers temperature, pressure, speed,
    // length, mass per the starter-spi units module.
    for q in ["temperature", "pressure", "speed", "length", "mass"] {
        assert!(out.contains(q), "missing quantity {q} in:\n{out}");
    }
    assert!(out.contains("canonical="), "expected canonical= column");
    assert!(out.contains("allowed=["), "expected allowed=[ column");
    app.shutdown().await;
}
