//! End-to-end: PUT an attachment, GET the JSON envelope back, then
//! GET the presigned URL it carries and assert the body matches.
//!
//! The test wires the same router shape `src/main.rs` does — a
//! consumer router plus the memory engine's presign router mounted
//! at `/blobs` — and binds a real local port so the presigned URL
//! returned by the engine is actually fetched over HTTP. That is
//! the load-bearing fact: the consumer never re-serves bytes;
//! the engine's own router does.

use std::sync::Arc;

use serde_json::Value;
use starter_blob_memory::{
    router::router as memory_router, MemoryBlobStore, MemoryBlobStoreConfig,
};
use starter_spi::blob::BackendId;
use starter_store_sqlite::pool;

#[path = "../src/server.rs"]
mod server;

#[tokio::test]
async fn upload_then_presigned_get_round_trip() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let db = pool::connect("sqlite::memory:").await.unwrap();
    server::migrate(&db).await.unwrap();

    let engine = MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new("memory:blobs-example-test"),
        public_base_url: format!("http://{addr}/blobs"),
    });
    let app = server::router(db, Arc::new(engine.clone()))
        .merge(axum::Router::new().nest("/blobs", memory_router(engine)));

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let payload = b"the-quick-brown-fox";
    let put = client
        .put(format!("http://{addr}/attachments/notes.txt"))
        .body(payload.to_vec())
        .send()
        .await
        .unwrap();
    assert!(put.status().is_success());
    let body: Value = put.json().await.unwrap();
    let id = body["id"].as_i64().unwrap();

    let get = client
        .get(format!("http://{addr}/attachments/by-id/{id}"))
        .send()
        .await
        .unwrap();
    assert!(get.status().is_success());
    let envelope: Value = get.json().await.unwrap();
    assert_eq!(envelope["name"], "notes.txt");
    let presigned = envelope["presigned_url"].as_str().unwrap().to_string();

    let bytes = client
        .get(&presigned)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(&bytes[..], payload);
}
