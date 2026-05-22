//! SCOPE TestWithoutNetwork smoke fixture.
//!
//! Runs the canonical put / get / head / delete / list / presign
//! surface against `starter-blob-memory` with no feature-flag
//! gymnastics and no sockets opened. If this test ever needs to
//! reach for a feature flag or a network shim, the memory engine
//! has stopped being the no-friction test seam the SCOPE relies
//! on.

use std::time::Duration;

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_memory::MemoryBlobStore;
use starter_spi::blob::{BlobKey, BlobStore, PresignOp, PutOptions};

#[tokio::test]
async fn full_surface_against_memory_no_network() {
    let s = MemoryBlobStore::new();

    let k = BlobKey::new("a/b.bin").unwrap();
    let r = s
        .put_bytes(
            &k,
            Bytes::from_static(b"network-not-required"),
            PutOptions::with_content_type("application/octet-stream"),
        )
        .await
        .unwrap();

    // head
    let meta = s.head(&r).await.unwrap();
    assert_eq!(meta.size, 20);

    // get (full)
    let bytes: Vec<Bytes> = s.get(&r, None).await.unwrap().try_collect().await.unwrap();
    let all: Vec<u8> = bytes.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(all, b"network-not-required");

    // list
    let page = s.list(None, None).await.unwrap();
    assert_eq!(page.items.len(), 1);

    // presign (in-process, no network)
    let url = s
        .presign(&r, PresignOp::Get, Duration::from_secs(30))
        .await
        .unwrap();
    assert!(url.url.starts_with("memory://"));

    // delete
    s.delete(&r).await.unwrap();
    assert!(s.head(&r).await.is_err());
}
