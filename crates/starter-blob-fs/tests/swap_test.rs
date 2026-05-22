//! SCOPE Swap-test fixture (stage-2 slice, fs side).
//!
//! Same shape as the memory-side fixture: a consumer function
//! takes `Arc<dyn BlobStore>` and is exercised against an fs
//! engine. The point is the function signature: a consumer must
//! be able to swap engines without re-typing call sites.

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_fs::{FsBlobStore, PresignKey};
use starter_spi::blob::{BlobKey, BlobStore, PutOptions};

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
async fn fs_engine_behind_arc_dyn_blob_store() {
    let dir = tempfile::tempdir().unwrap();
    let store: Arc<dyn BlobStore> =
        Arc::new(FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap());
    put_then_read(store, "scratch/x.bin", b"on-disk-bytes").await;
}

// The compose-test fixture lives in
// `crates/starter-blob-compose/tests/compose_test.rs` so this
// crate keeps a slim dep tree (a consumer who only wants the fs
// engine never picks up the combinator crate). Stage 5 hoists it
// into the workspace-level `smoke-tests` crate alongside the rest
// of the SCOPE smoke fixtures.
