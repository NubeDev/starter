# starter-blob-garage

Garage-aware [`BlobStore`](../starter-spi/src/blob). The data plane
delegates to [`starter-blob-s3`](../starter-blob-s3/README.md) over
Garage's S3-compatible endpoint; this crate layers Garage-specific
operator surface on top:

- bucket lifecycle via the admin API (`POST /v1/bucket`,
  `DELETE /v1/bucket/{id}`),
- per-bucket access-key minting (`POST /v1/key`, `POST /v1/bucket/allow`),
- cluster health (`GET /v1/health`) surfaced as a typed
  `ClusterStatus`,
- cluster-layout introspection (`GET /v1/layout`) probed at
  startup so a misconfigured cluster fails on construction rather
  than on the first `put`.

## Stand-up

The repo ships a reference compose:

```sh
docker compose -f docker/docker-compose.garage.yml up -d
```

The init container creates a `starter-test` bucket and mints a key
named `starter-test-key`. The credentials are printed in the init
container's logs.

## Quick start

```rust
use starter_blob_garage::{
    GarageAdmin, GarageBlobStore, S3BlobStoreConfig, S3Credentials,
};
use starter_spi::blob::BackendId;

let admin = GarageAdmin::new("http://localhost:3903", admin_token)?;
let layout = admin.layout().await?;            // operator-visible introspection
let bucket = admin.create_bucket("my-bucket").await?;
let key    = admin.create_key("my-app").await?;
admin.allow_key(&bucket.id, &key.access_key_id).await?;

let s3 = S3BlobStoreConfig::new(
    BackendId::new("garage:test"),
    "my-bucket",
    "garage",
)
.endpoint_url("http://localhost:3900")
.force_path_style(true);
let creds = S3Credentials {
    access_key_id: key.access_key_id,
    secret_access_key: key.secret_access_key,
    session_token: None,
};
let store = GarageBlobStore::open(s3, creds, &admin).await?;
```

## Licensing

`starter-blob-garage` is **MIT OR Apache-2.0**. The crate reaches
Garage only over its HTTP S3 and admin APIs — **no Garage Rust
crate is linked**. Verify with:

```sh
cargo tree -p starter-blob-garage | grep -i garage
# expected: only `starter-blob-garage` itself appears
```

Garage upstream is AGPL-3.0. Starter consumers reach Garage over
the wire; the AGPL boundary stops at the network seam, so consumers
of this crate do **not** inherit AGPL obligations.
