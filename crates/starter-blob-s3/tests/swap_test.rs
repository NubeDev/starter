//! SCOPE Swap-test fixture (stage-3 slice, S3 side).
//!
//! This file is compile-only: a real S3 endpoint is not available
//! in `cargo test --workspace`. The integration tests under
//! `tests/integration.rs` (gated by the `integration-tests` feature)
//! exercise an actual MinIO / Garage endpoint when the CI lane
//! brings one up.
//!
//! The point of this file is the function signature: a consumer
//! that takes `Arc<dyn BlobStore>` must be able to swap from the
//! `-fs` engine to `-s3` with one wiring change, and the function
//! body never changes.

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_s3::{S3BlobStore, S3BlobStoreConfig};
use starter_spi::blob::{BackendId, BlobKey, BlobStore, PutOptions};

#[allow(dead_code)]
async fn put_then_read(store: Arc<dyn BlobStore>, key: &str, payload: &[u8]) {
    let k = BlobKey::new(key).unwrap();
    let r = store
        .put_bytes(&k, Bytes::copy_from_slice(payload), PutOptions::default())
        .await
        .unwrap();
    let stream = store.get(&r, None).await.unwrap();
    let chunks: Vec<Bytes> = stream.try_collect().await.unwrap();
    let got: Vec<u8> = chunks.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(got, payload);
}

#[allow(dead_code)]
async fn build_for_swap() -> Arc<dyn BlobStore> {
    let cfg = S3BlobStoreConfig::new(BackendId::new("s3:test"), "test-bucket", "us-east-1");
    let store = S3BlobStore::open(cfg).await.unwrap();
    Arc::new(store)
}

#[test]
fn s3_engine_is_object_safe_behind_arc_dyn_blob_store() {
    // Compile-time fact only; real swap is exercised by the
    // integration tests against a live endpoint.
    let _: fn() -> _ = build_for_swap;
    let _ = put_then_read; // ensures the consumer signature stays
                           // `Arc<dyn BlobStore>`; if `S3BlobStore`
                           // stopped being object-safe this test
                           // file would fail to compile.
}

// TODO(stage 4): compose-test — wrap S3BlobStore in `Namespaced`
// and `Tiered` from `starter-blob-compose`, re-run against the
// wrapped store, and assert no consumer signature changes.
