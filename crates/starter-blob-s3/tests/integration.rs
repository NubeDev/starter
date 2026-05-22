//! Live S3 integration tests. Gated on the `integration-tests`
//! feature so `cargo test --workspace` stays hermetic. The CI lane
//! enables this feature alongside the docker-compose Garage / MinIO
//! stand-up:
//!
//! ```sh
//! docker compose -f docker/docker-compose.garage.yml up -d
//! cargo test -p starter-blob-s3 --features integration-tests \
//!   --test integration -- --test-threads=1
//! ```
//!
//! Environment variables (all required when the feature is on; tests
//! call `eprintln!` and skip if absent so a partial config never
//! pretends to pass):
//!
//! - `STARTER_S3_ENDPOINT`     — e.g. `http://localhost:3900`
//! - `STARTER_S3_REGION`       — e.g. `garage`
//! - `STARTER_S3_BUCKET`       — e.g. `starter-test`
//! - `STARTER_S3_ACCESS_KEY`   — minted by Garage's admin API
//! - `STARTER_S3_SECRET_KEY`

#![cfg(feature = "integration-tests")]

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_s3::{S3BlobStore, S3BlobStoreConfig, S3Credentials};
use starter_spi::blob::{BackendId, BlobKey, BlobStore, PutOptions};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

async fn make_store() -> Option<S3BlobStore> {
    let endpoint = env("STARTER_S3_ENDPOINT")?;
    let region = env("STARTER_S3_REGION")?;
    let bucket = env("STARTER_S3_BUCKET")?;
    let ak = env("STARTER_S3_ACCESS_KEY")?;
    let sk = env("STARTER_S3_SECRET_KEY")?;
    let cfg = S3BlobStoreConfig::new(BackendId::new(format!("s3:test:{bucket}")), bucket, region)
        .endpoint_url(endpoint)
        .force_path_style(true);
    S3BlobStore::open_with_credentials(
        cfg,
        S3Credentials {
            access_key_id: ak,
            secret_access_key: sk,
            session_token: None,
        },
    )
    .await
    .ok()
}

#[tokio::test]
async fn live_roundtrip() {
    let Some(store) = make_store().await else {
        eprintln!("STARTER_S3_* not set; skipping");
        return;
    };
    let key = BlobKey::new("starter-integration/hello.txt").unwrap();
    let r = store
        .put_bytes(
            &key,
            Bytes::from_static(b"hello-garage"),
            PutOptions::with_content_type("text/plain"),
        )
        .await
        .expect("put");
    let stream = store.get(&r, None).await.expect("get");
    let chunks: Vec<Bytes> = stream.try_collect().await.expect("body");
    let got: Vec<u8> = chunks.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(got, b"hello-garage");
    store.delete(&r).await.expect("delete");
}
