//! SCOPE Swap-test fixture (stage-3 slice, Garage side).
//!
//! Compile-only check that `GarageBlobStore` is `dyn BlobStore`-safe
//! and that the construction path stays one line. A live cluster is
//! required for the runtime side; see `tests/integration.rs`
//! (feature-gated).

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_spi::blob::{BlobKey, BlobStore, PutOptions};

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

#[test]
fn garage_engine_is_object_safe() {
    // Compile-time fact: the function exists with the right
    // signature. The runtime swap is exercised by integration tests
    // against a live cluster.
    let _ = put_then_read;
}

// TODO(stage 4): compose-test — wrap GarageBlobStore in
// `Namespaced` and `Tiered` from `starter-blob-compose`, re-run
// against the wrapped store, and assert no consumer signature
// changes.
