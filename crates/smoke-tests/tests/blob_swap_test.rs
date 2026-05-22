//! SCOPE smoke 1 — **SwapTest**.
//!
//! A consumer-style function takes `Arc<dyn BlobStore>` and exercises
//! the canonical put / head / get / list / delete surface. The same
//! function compiles and behaves identically against
//! `starter-blob-memory` and `starter-blob-fs` — the contract is
//! that a deployment swaps engines with a one-line construction
//! change.
//!
//! The smoke runs against the test/dev engines (`memory`, `fs`) so it
//! is hermetic; `starter-blob-s3` / `starter-blob-garage` add nothing
//! to the trait contract that the test/dev engines do not already
//! exercise.

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_fs::{FsBlobStore, PresignKey};
use starter_blob_memory::MemoryBlobStore;
use starter_spi::blob::{BlobKey, BlobStore, PutOptions};

async fn consumer_round_trip(store: Arc<dyn BlobStore>) {
    let key = BlobKey::new("scope/swap-test.bin").unwrap();
    let payload = Bytes::from_static(b"swap-test-payload");
    let r = store
        .put_bytes(
            &key,
            payload.clone(),
            PutOptions::with_content_type("application/octet-stream"),
        )
        .await
        .unwrap();
    assert_eq!(r.size(), payload.len() as u64);

    let meta = store.head(&r).await.unwrap();
    assert_eq!(meta.size, payload.len() as u64);

    let chunks: Vec<Bytes> = store
        .get(&r, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let got: Vec<u8> = chunks.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(got, payload);

    let page = store.list(None, None).await.unwrap();
    assert!(!page.items.is_empty());

    store.delete(&r).await.unwrap();
}

#[tokio::test]
async fn swap_memory_engine() {
    let store: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::new());
    consumer_round_trip(store).await;
}

#[tokio::test]
async fn swap_fs_engine() {
    let dir = tempfile::tempdir().unwrap();
    let store: Arc<dyn BlobStore> =
        Arc::new(FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap());
    consumer_round_trip(store).await;
}
