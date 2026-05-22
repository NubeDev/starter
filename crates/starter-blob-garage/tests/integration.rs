//! Live Garage integration tests. Gated on `integration-tests` so
//! the workspace `cargo test` stays hermetic.
//!
//! Brings the cluster up with
//! `docker compose -f docker/docker-compose.garage.yml up -d` and
//! reads credentials from `STARTER_GARAGE_*` env vars (the init
//! container in the compose prints them).

#![cfg(feature = "integration-tests")]

use bytes::Bytes;
use futures::TryStreamExt;
use starter_blob_garage::{
    ClusterStatus, GarageAdmin, GarageBlobStore, S3BlobStoreConfig, S3Credentials,
};
use starter_spi::blob::{BackendId, BlobKey, BlobStore, PutOptions};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

#[tokio::test]
async fn live_health_and_roundtrip() {
    let (Some(admin_url), Some(admin_token), Some(s3_endpoint), Some(bucket), Some(ak), Some(sk)) = (
        env("STARTER_GARAGE_ADMIN_URL"),
        env("STARTER_GARAGE_ADMIN_TOKEN"),
        env("STARTER_GARAGE_S3_ENDPOINT"),
        env("STARTER_GARAGE_BUCKET"),
        env("STARTER_GARAGE_ACCESS_KEY"),
        env("STARTER_GARAGE_SECRET_KEY"),
    ) else {
        eprintln!("STARTER_GARAGE_* not set; skipping");
        return;
    };

    let admin = GarageAdmin::new(&admin_url, admin_token).unwrap();
    let health = admin.health().await.expect("health");
    assert!(
        !matches!(health.status, ClusterStatus::Unavailable),
        "cluster unavailable at probe: {:?}",
        health.status
    );

    let cfg = S3BlobStoreConfig::new(
        BackendId::new(format!("garage:test:{bucket}")),
        bucket,
        "garage",
    )
    .endpoint_url(s3_endpoint)
    .force_path_style(true);
    let creds = S3Credentials {
        access_key_id: ak,
        secret_access_key: sk,
        session_token: None,
    };
    let store = GarageBlobStore::open(cfg, creds, &admin)
        .await
        .expect("open");
    assert!(store.layout().node_count >= 1);

    let key = BlobKey::new("starter-garage-integration/hi.txt").unwrap();
    let r = store
        .put_bytes(
            &key,
            Bytes::from_static(b"hi-garage"),
            PutOptions::with_content_type("text/plain"),
        )
        .await
        .expect("put");
    let stream = store.get(&r, None).await.expect("get");
    let chunks: Vec<Bytes> = stream.try_collect().await.expect("body");
    let got: Vec<u8> = chunks.iter().flat_map(|b| b.iter().copied()).collect();
    assert_eq!(got, b"hi-garage");
    store.delete(&r).await.expect("delete");
}
