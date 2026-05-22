//! SCOPE Swap-test fixture (stage-2 slice).
//!
//! Stage 2 only covers the two test/dev engines, so the swap test
//! here is `memory` ↔ `memory`-with-a-different-config: the *same*
//! consumer function takes `Arc<dyn BlobStore>` and is exercised
//! against both. Stage 3 extends this to `fs → s3 → garage`; the
//! stage-5 workspace-level smoke test layers the full matrix.
//!
//! The point of having the fixture *here* is that flipping the
//! engine in front of an unchanged consumer must already be a
//! compile-time + runtime no-op, so future stages cannot regress
//! the property.

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_memory::{MemoryBlobStore, MemoryBlobStoreConfig};
use starter_spi::blob::{BackendId, BlobKey, BlobStore, PutOptions};

/// Consumer-style function: stores a tagged blob, reads it back,
/// asserts the bytes match. Takes `Arc<dyn BlobStore>` exactly the
/// way the SCOPE's SwapTest demands — the consumer never names a
/// concrete engine.
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

#[tokio::test]
async fn swap_two_memory_engines_compiles_and_behaves_identically() {
    let a: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new("memory:a"),
        ..Default::default()
    }));
    let b: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new("memory:b"),
        ..Default::default()
    }));
    put_then_read(a, "k", b"payload-a").await;
    put_then_read(b, "k", b"payload-b").await;
}

// The compose-test fixture lives in
// `crates/starter-blob-compose/tests/compose_test.rs` so this
// crate keeps a slim dep tree (a consumer who only wants the
// memory engine never picks up the combinator crate). Stage 5
// hoists it into the workspace-level `smoke-tests` crate
// alongside the rest of the SCOPE smoke fixtures.
