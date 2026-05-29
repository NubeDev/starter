//! Cleanup endpoints — Stage 5 (P4).
//!
//! Exercises the HTTP surface:
//! - `GET  /extensions/<id>/cleanup` dry-run manifest,
//! - `DELETE /extensions/<id>?purge=true` full purge (removes the
//!   enablement row outright, not a flip to `Disabled`),
//! - idempotent purge on an already-uninstalled (ghost-row) id: a 200
//!   `cleanup.succeeded`, never a 404.
//!
//! The built-in `EnablementRowProvider` auto-registers against the same
//! in-memory store the admin uses, so a purge reaches the persistence row
//! with no rubix wiring.

use std::io::Write;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::Value;
use starter_ext_host::ExtensionRegistry;
use starter_ext_server::{
    router, EnablementState, EnablementStore, ExtensionAdmin, InMemoryEnablementStore,
};
use starter_ext_spi::ExtensionId;
use tempfile::tempdir;
use tower::ServiceExt;

const BUNDLE_YAML: &str = r#"
v: 1
id: com.acme.purgeme
version: 0.1.0
display_name: "Purge Me"
description_file: docs/README.md
authors: ["ap@nube-io.com"]
runtime:
  kind: builtin
  crate_name: purge-builtin
contributes:
  tools:
    - id: com.acme.purgeme.echo
      input_schema: schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/echo.md
"#;

fn build_tarball() -> Vec<u8> {
    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(gz);
    let bytes = BUNDLE_YAML.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "block.yaml", bytes).unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

fn multipart_body(payload: &[u8], boundary: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    write!(
        out,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
         filename=\"ext.tgz\"\r\nContent-Type: application/gzip\r\n\r\n"
    )
    .unwrap();
    out.extend_from_slice(payload);
    write!(out, "\r\n--{boundary}--\r\n").unwrap();
    out
}

fn build_admin(dir: std::path::PathBuf) -> (ExtensionAdmin, Arc<InMemoryEnablementStore>) {
    let mut reg = ExtensionRegistry::new();
    reg.seal();
    let store = Arc::new(InMemoryEnablementStore::new());
    let admin = ExtensionAdmin::builder(Arc::new(reg))
        .with_enablement_store(store.clone() as Arc<dyn EnablementStore>)
        .with_installs_dir(dir)
        .build();
    (admin, store)
}

async fn install(app: &axum::Router<()>) {
    let boundary = "----purgetest";
    let body = multipart_body(&build_tarball(), boundary);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/extensions/install")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "install must succeed");
}

#[tokio::test]
async fn cleanup_dry_run_lists_enablement_row() {
    let tmp = tempdir().unwrap();
    let (admin, _store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);
    install(&app).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/extensions/com.acme.purgeme/cleanup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["id"], "com.acme.purgeme");
    let items = json["items"].as_array().unwrap();
    assert!(
        items.iter().any(|i| i["kind"] == "enablement_row"),
        "dry run must surface the enablement row: {json}"
    );
    // The store row is untouched by a dry run.
    // (purge happens only via DELETE ?purge=true.)
}

#[tokio::test]
async fn purge_deletes_row_and_is_idempotent() {
    let tmp = tempdir().unwrap();
    let (admin, store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);
    install(&app).await;

    let ext_id = ExtensionId::new("com.acme.purgeme").unwrap();
    assert_eq!(
        store.get(&ext_id).await.unwrap(),
        Some(EnablementState::Enabled),
        "install must persist an Enabled row"
    );

    // First purge: removes the row outright (not a flip to Disabled).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/extensions/com.acme.purgeme?purge=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "cleanup.succeeded");
    let removed = json["removed"].as_array().unwrap();
    assert!(
        removed.iter().any(|i| i["kind"] == "enablement_row"),
        "first purge must report the row removed: {json}"
    );
    assert!(
        store.get(&ext_id).await.unwrap().is_none(),
        "row must be DELETEd, not left as a Disabled ghost"
    );

    // Second purge on the now-ghost id: still 200 cleanup.succeeded, never
    // a 404, with an empty removed set (nothing left to reclaim).
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/extensions/com.acme.purgeme?purge=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "idempotent purge must not 404"
    );
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "cleanup.succeeded");
    assert!(
        json["removed"].as_array().unwrap().is_empty(),
        "nothing left to remove on the second purge: {json}"
    );
}

#[tokio::test]
async fn non_purge_delete_keeps_disabled_row() {
    let tmp = tempdir().unwrap();
    let (admin, store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);
    install(&app).await;

    let ext_id = ExtensionId::new("com.acme.purgeme").unwrap();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/extensions/com.acme.purgeme")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "uninstall.succeeded");
    // Default (?purge=false) keeps today's behaviour: the row is flipped to
    // Disabled, not removed.
    assert_eq!(
        store.get(&ext_id).await.unwrap(),
        Some(EnablementState::Disabled)
    );
}
