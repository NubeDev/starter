//! Tarball install/uninstall roundtrip — Phase D.1.
//!
//! Build a gzipped tar bundle in-memory, POST it as multipart to
//! `/extensions/install`, assert the bundle lands on disk under the
//! configured extensions root, then DELETE `/extensions/<id>` and
//! assert the bundle directory is gone and the persistence row reads
//! `Disabled`.
//!
//! The router is built with the unauthed `router(admin)` variant — the
//! authz sandwich is the parent workspace's responsibility (see
//! `rubix-agent` boot) and would only add noise to a lifecycle smoke
//! test.

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
id: com.acme.installed
version: 0.1.0
display_name: "Installed"
description_file: docs/README.md
authors: ["ap@nube-io.com"]
runtime:
  kind: builtin
  crate_name: installed-builtin
contributes:
  tools:
    - id: com.acme.installed.echo
      input_schema: schemas/in.json
      output_schema: schemas/out.json
      description_file: docs/echo.md
"#;

fn build_tarball(top_dir: Option<&str>) -> Vec<u8> {
    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(gz);
    let path_yaml = match top_dir {
        Some(top) => format!("{top}/block.yaml"),
        None => "block.yaml".to_owned(),
    };
    let bytes = BUNDLE_YAML.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, &path_yaml, bytes).unwrap();
    let gz = tar.into_inner().unwrap();
    gz.finish().unwrap()
}

fn multipart_body(field: &str, filename: &str, payload: &[u8], boundary: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    write!(
        out,
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; \
         filename=\"{filename}\"\r\nContent-Type: application/gzip\r\n\r\n"
    )
    .unwrap();
    out.extend_from_slice(payload);
    write!(out, "\r\n--{boundary}--\r\n").unwrap();
    out
}

fn build_admin(extensions_dir: std::path::PathBuf) -> (ExtensionAdmin, Arc<InMemoryEnablementStore>) {
    let mut reg = ExtensionRegistry::new();
    reg.seal();
    let store = Arc::new(InMemoryEnablementStore::new());
    let admin = ExtensionAdmin::builder(Arc::new(reg))
        .with_enablement_store(store.clone() as Arc<dyn EnablementStore>)
        .with_extensions_dir(extensions_dir)
        .build();
    (admin, store)
}

#[tokio::test]
async fn install_then_uninstall_roundtrip() {
    let tmp = tempdir().unwrap();
    let (admin, store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);

    let tarball = build_tarball(None);
    let boundary = "----lifecycletest";
    let body = multipart_body("file", "ext.tgz", &tarball, boundary);

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
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "install.succeeded");
    assert_eq!(json["id"], "com.acme.installed");

    // Bundle is on disk under the configured root.
    let bundle_dir = tmp.path().join("com.acme.installed");
    assert!(bundle_dir.join("block.yaml").exists(), "block.yaml must exist after install");

    // Persistence row marked Enabled.
    let ext_id = ExtensionId::new("com.acme.installed").unwrap();
    assert_eq!(
        store.get(&ext_id).await.unwrap(),
        Some(EnablementState::Enabled)
    );

    // Now uninstall.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/extensions/com.acme.installed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "uninstall must succeed");
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "uninstall.succeeded");

    // Bundle is gone, persistence row flipped to Disabled.
    assert!(!bundle_dir.exists(), "bundle directory must be removed");
    assert_eq!(
        store.get(&ext_id).await.unwrap(),
        Some(EnablementState::Disabled)
    );
}

#[tokio::test]
async fn install_invalid_manifest_returns_code() {
    let tmp = tempdir().unwrap();
    let (admin, _store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);

    // Tarball with garbage block.yaml.
    let buf = Vec::new();
    let gz = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(gz);
    let bytes = b"not-a-valid-manifest: ::: bad\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "block.yaml", &bytes[..]).unwrap();
    let payload = tar.into_inner().unwrap().finish().unwrap();
    let boundary = "----badmanifest";
    let body = multipart_body("file", "ext.tgz", &payload, boundary);

    let resp = app
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
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "install.invalid_manifest");
}

#[tokio::test]
async fn uninstall_missing_returns_not_found_code() {
    let tmp = tempdir().unwrap();
    let (admin, _store) = build_admin(tmp.path().to_path_buf());
    let app = router::<()>(admin);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/extensions/com.acme.ghost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "uninstall.not_found");
}
