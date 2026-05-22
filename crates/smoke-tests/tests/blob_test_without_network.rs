//! SCOPE smoke 3 — **TestWithoutNetwork**.
//!
//! The full BlobStore integration surface (put / put_stream / head /
//! get / get-with-range / list / delete / presign) runs end-to-end
//! against `starter-blob-memory` with no feature-flag gymnastics
//! and no network. The point: a consumer running CI without S3
//! credentials gets coverage of every method on the trait.
//!
//! The compose combinators are exercised in `blob_compose_test.rs`;
//! the presign axum router is exercised in
//! `examples/blobs/tests/round_trip.rs`. Together those three tests
//! mean the consumer never needs a Garage stand-up to assert their
//! own code is correct against the seam.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::stream::{self, StreamExt, TryStreamExt};
use starter_blob_memory::MemoryBlobStore;
use starter_spi::blob::{BlobKey, BlobRange, BlobStore, PresignOp, PutOptions};

async fn collect(
    s: futures::stream::BoxStream<'static, Result<Bytes, starter_spi::blob::BlobError>>,
) -> Vec<u8> {
    let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
    chunks.iter().flat_map(|b| b.iter().copied()).collect()
}

#[tokio::test]
async fn full_trait_surface_against_memory_engine() {
    let store: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::new());

    // put_bytes
    let k1 = BlobKey::new("a/one.bin").unwrap();
    let r1 = store
        .put_bytes(
            &k1,
            Bytes::from_static(b"hello-world"),
            PutOptions::default(),
        )
        .await
        .unwrap();

    // put_stream
    let chunks = vec![
        Ok(Bytes::from_static(b"strea")),
        Ok(Bytes::from_static(b"med")),
    ];
    let k2 = BlobKey::new("a/two.bin").unwrap();
    let r2 = store
        .put_stream(&k2, stream::iter(chunks).boxed(), PutOptions::default())
        .await
        .unwrap();

    // head
    let meta = store.head(&r1).await.unwrap();
    assert_eq!(meta.size, 11);

    // get whole
    let body = collect(store.get(&r1, None).await.unwrap()).await;
    assert_eq!(body, b"hello-world");

    // get range
    let body = collect(
        store
            .get(&r1, Some(BlobRange::new(6, 10).unwrap()))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(body, b"world");

    // list
    let page = store.list(None, None).await.unwrap();
    assert_eq!(page.items.len(), 2);

    // presign (engine returns its own URL — no network)
    let url = store
        .presign(&r2, PresignOp::Get, Duration::from_secs(5))
        .await
        .unwrap();
    assert!(url.url.contains("token="));

    // delete is idempotent
    store.delete(&r1).await.unwrap();
    store.delete(&r1).await.unwrap();
}
