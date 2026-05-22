//! SCOPE compose-test smoke fixture.
//!
//! Stage 2 left a TODO marker in `starter-blob-memory/tests/swap_test.rs`
//! and `starter-blob-fs/tests/swap_test.rs` saying "stage 4 wraps these
//! in Namespaced + Tiered + Mirrored + ReadThroughCache and re-runs
//! `put_then_read` against the wrapped Arc<dyn BlobStore>; assert no
//! consumer signature changes."
//!
//! This file is that fixture. It deliberately lives in
//! `starter-blob-compose` rather than in either engine crate so the
//! engine crates do not pick up a dev-dep on the combinator crate
//! (cost-to-skip: an engine consumer who never composes still gets
//! the slim dep tree). Stage 5 will hoist this into the
//! workspace-level `smoke-tests` crate alongside the rest of the
//! SCOPE smoke fixtures.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_compose::{
    MirrorMode, Mirrored, Namespaced, ReadThroughCache, Tiered, TieredPolicy,
};
use starter_blob_fs::{FsBlobStore, PresignKey};
use starter_blob_memory::{MemoryBlobStore, MemoryBlobStoreConfig};
use starter_spi::blob::{BackendId, BlobKey, BlobStore, PutOptions};

/// Consumer-style function: takes `Arc<dyn BlobStore>`, never names
/// a concrete engine OR combinator. Stage 2's `swap_test.rs`
/// fixtures use the same signature — the point of *this* test is
/// that the function does not need any change to run against a
/// composed store.
async fn put_then_read(store: Arc<dyn BlobStore>, key: &str, payload: &[u8]) {
    let k = BlobKey::new(key).unwrap();
    let r = store
        .put_bytes(&k, Bytes::copy_from_slice(payload), PutOptions::default())
        .await
        .unwrap();
    let chunks: Vec<Bytes> = store
        .get(&r, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let got: Vec<u8> = chunks.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(got, payload);
    let meta = store.head(&r).await.unwrap();
    assert_eq!(meta.size as usize, payload.len());
}

fn mem(id: &str) -> Arc<MemoryBlobStore> {
    Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new(id),
        ..Default::default()
    }))
}

#[tokio::test]
async fn memory_wrapped_in_namespaced_and_tiered_obeys_consumer_contract() {
    let hot = mem("hot");
    let cold = mem("cold");
    let tiered = Arc::new(Tiered::new(
        hot,
        cold,
        TieredPolicy {
            demote_above_bytes: Some(8),
            ..Default::default()
        },
    ));
    let store: Arc<dyn BlobStore> = Arc::new(Namespaced::new(tiered, "tenant-7/").unwrap());
    put_then_read(store.clone(), "small.bin", b"tiny").await;
    put_then_read(store, "bigger.bin", b"this-is-larger-than-eight").await;
}

#[tokio::test]
async fn fs_wrapped_in_mirrored_obeys_consumer_contract() {
    let dir = tempfile::tempdir().unwrap();
    let fs: Arc<dyn BlobStore> =
        Arc::new(FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap());
    let mirror = mem("mirror");
    let store: Arc<dyn BlobStore> = Arc::new(
        Mirrored::builder(fs)
            .mirror(mirror.clone())
            .mode(MirrorMode::Sync)
            .build(),
    );
    put_then_read(store, "doc/x.bin", b"on-disk-with-mirror").await;
    // Sync mirror means the mirror has the bytes too.
    let mirror_page = mirror.list(None, None).await.unwrap();
    assert_eq!(mirror_page.items.len(), 1);
}

#[tokio::test]
async fn read_through_cache_serves_after_first_read() {
    let source = mem("source");
    let cache = mem("cache");
    let rtc: Arc<dyn BlobStore> = Arc::new(ReadThroughCache::new(
        source.clone(),
        cache.clone(),
        Some(Duration::from_secs(60)),
    ));
    put_then_read(rtc.clone(), "cached.bin", b"please-cache-me").await;
    // After the first read, the cache holds the bytes.
    assert_eq!(cache.list(None, None).await.unwrap().items.len(), 1);
}

#[tokio::test]
async fn combinators_nest_three_deep_without_signature_change() {
    // The whole point of the trait: a 3-deep stack still hands the
    // consumer a single `Arc<dyn BlobStore>`. No generics in the
    // consumer signature.
    let hot = mem("hot");
    let cold = mem("cold");
    let tiered = Arc::new(Tiered::new(hot, cold, TieredPolicy::default()));
    let scoped = Arc::new(Namespaced::new(tiered, "feature-x/").unwrap());
    let mirror = mem("mirror");
    let stack: Arc<dyn BlobStore> = Arc::new(
        Mirrored::builder(scoped)
            .mirror(mirror)
            .mode(MirrorMode::Sync)
            .build(),
    );
    put_then_read(stack, "anything", b"three-deep").await;
}
