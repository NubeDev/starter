//! End-to-end tests for the admin slice.
//!
//! Each test builds a temp bundle directory, runs the kernel loader,
//! constructs an [`ExtensionAdmin`] without any supervisor (builtin
//! flavour — no spawn needed), and exercises the router via
//! `tower::ServiceExt::oneshot`. The supervisor-bound endpoints
//! (`/events`) are smoke-tested with an in-process supervisor handle
//! built from a stub when we need one — but for the admin slice the
//! "no supervisor → 404 on /events" behaviour is the primary contract.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use serde_json::Value;
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_server::{router, ExtensionAdmin};
use tempfile::tempdir;
use tower::ServiceExt;

const HELLO_BUNDLE: &str = r#"
v: 1
id: com.acme.hello
version: 0.1.0
display_name: "Hello"
description_file: docs/README.md
authors: ["ap@nube-io.com"]
runtime:
  kind: builtin
  crate_name: hello-builtin
contributes:
  tools:
    - id: com.acme.hello.echo
      input_schema: schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/echo.md
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: HelloPanel, module: "./Panel", slot: "sidebar" }
"#;

fn write_bundle(root: &std::path::Path, id: &str, body: &str) {
    let dir = root.join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("block.yaml"), body).unwrap();
}

fn build_admin() -> (ExtensionAdmin, tempfile::TempDir) {
    let tmp = tempdir().unwrap();
    write_bundle(tmp.path(), "com.acme.hello", HELLO_BUNDLE);
    // Stage a UI asset so the ETag-cached serving path is exercised.
    let ui_dir = tmp.path().join("com.acme.hello").join("ui");
    std::fs::create_dir_all(&ui_dir).unwrap();
    std::fs::write(ui_dir.join("remoteEntry.js"), b"export const x = 1;\n").unwrap();

    let recs = Loader::scan(tmp.path()).validate_all();
    let mut reg = ExtensionRegistry::new();
    Loader::commit(recs, &mut reg);
    reg.seal();
    let admin = ExtensionAdmin::builder(Arc::new(reg)).build();
    (admin, tmp)
}

#[tokio::test]
async fn list_returns_validated_extension() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/extensions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "com.acme.hello");
    assert_eq!(arr[0]["state"], "validated");
    assert_eq!(arr[0]["runtime_kind"], "builtin");
    assert_eq!(arr[0]["enabled"], "enabled");
}

#[tokio::test]
async fn detail_404_for_unknown_id() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/extensions/com.nope.absent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn detail_returns_manifest() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/extensions/com.acme.hello")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], "com.acme.hello");
    assert_eq!(json["manifest"]["id"], "com.acme.hello");
    assert_eq!(json["enabled"], "enabled");
    assert_eq!(json["events_cursor"], 0);
}

#[tokio::test]
async fn disable_then_enable_round_trips() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let disable_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/extensions/com.acme.hello/disable")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disable_resp.status(), StatusCode::OK);
    let body = to_bytes(disable_resp.into_body(), 1024).await.unwrap();
    let j: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(j["enabled"], "disabled");

    let enable_resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/extensions/com.acme.hello/enable")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enable_resp.status(), StatusCode::OK);
    let body = to_bytes(enable_resp.into_body(), 1024).await.unwrap();
    let j: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(j["enabled"], "enabled");
}

#[tokio::test]
async fn events_404_when_no_supervisor() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/extensions/com.acme.hello/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ui_serves_with_strong_etag_and_304_revalidation() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/extensions/com.acme.hello/ui/remoteEntry.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let etag = resp
        .headers()
        .get(header::ETAG)
        .expect("etag present")
        .to_str()
        .unwrap()
        .to_string();
    assert!(etag.starts_with('"') && etag.ends_with('"'));
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/javascript"
    );

    // Second request with If-None-Match → 304.
    let resp2 = app
        .oneshot(
            Request::builder()
                .uri("/extensions/com.acme.hello/ui/remoteEntry.js")
                .header(header::IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        resp2.headers().get(header::ETAG).unwrap().to_str().unwrap(),
        etag.as_str()
    );
}

#[tokio::test]
async fn ui_rejects_parent_traversal() {
    let (admin, _tmp) = build_admin();
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/extensions/com.acme.hello/ui/..%2f..%2fblock.yaml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Either 403 from safe_join or 404 from axum's path normalisation;
    // never 200 (the manifest must not be reachable through ui/).
    assert_ne!(resp.status(), StatusCode::OK);
}
