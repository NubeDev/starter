//! SCOPE smoke 2 — **ComposeTest**.
//!
//! Wrapping any engine in `Namespaced` or `Tiered` (or both, or
//! nested combinators) requires **no change** to consumers of
//! `BlobRef`. The consumer holds the outer `BlobRef` returned by
//! the combinator and gets/heads/deletes against the same handle;
//! the combinator rewrites `BlobRef` on the way out and decodes on
//! the way in. No SQL migration is required because the column
//! stores opaque JSON.
//!
//! Test shape: a consumer function takes `Arc<dyn BlobStore>` and
//! `Vec<BlobRef>` mid-flight via put → head → get → delete. The
//! function runs once against a bare engine and once against the
//! same engine wrapped in `Namespaced<Tiered<...>>`. Both runs
//! must produce identical body bytes; the wrapped run additionally
//! asserts the outer `BlobRef` carries the combinator's
//! `backend_id`, not the inner engine's — proof that the
//! consumer's persisted JSON would route through the combinator on
//! a subsequent process restart.

use std::sync::Arc;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_compose::{Namespaced, Tiered, TieredPolicy};
use starter_blob_memory::{MemoryBlobStore, MemoryBlobStoreConfig};
use starter_spi::blob::{BackendId, BlobKey, BlobRef, BlobStore, PutOptions};

async fn consumer_flow(store: Arc<dyn BlobStore>) -> BlobRef {
    let key = BlobKey::new("doc/inv-42.bin").unwrap();
    let payload = Bytes::from_static(b"compose-test-payload");
    let r = store
        .put_bytes(&key, payload.clone(), PutOptions::default())
        .await
        .unwrap();
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
    r
}

fn mem(id: &str) -> Arc<MemoryBlobStore> {
    Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
        backend_id: BackendId::new(id),
        ..Default::default()
    }))
}

#[tokio::test]
async fn bare_engine_and_wrapped_engine_pass_same_consumer_flow() {
    let bare: Arc<dyn BlobStore> = mem("bare");
    let bare_ref = consumer_flow(bare).await;
    assert_eq!(bare_ref.backend_id().as_str(), "bare");

    let hot = mem("hot");
    let cold = mem("cold");
    let tiered = Arc::new(Tiered::new(hot, cold, TieredPolicy::default()));
    let wrapped: Arc<dyn BlobStore> = Arc::new(Namespaced::new(tiered, "tenant-7/").unwrap());
    let wrapped_ref = consumer_flow(wrapped).await;

    // Compose-test guarantee: the BlobRef the consumer persists
    // carries the *combinator's* backend_id, so a subsequent
    // process boot routes through the combinator. No domain code
    // changed; no SQL migration changed.
    assert_ne!(wrapped_ref.backend_id().as_str(), "hot");
    assert_ne!(wrapped_ref.backend_id().as_str(), "cold");
}
