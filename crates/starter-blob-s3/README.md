# starter-blob-s3

S3-backed [`BlobStore`](../starter-spi/src/blob) implementation. Wraps the
official [`aws-sdk-s3`](https://crates.io/crates/aws-sdk-s3) client so the
same engine targets:

- AWS S3
- MinIO (`force_path_style = true`)
- Cloudflare R2, Backblaze B2, Wasabi (any S3-compatible endpoint)
- Garage (via the sibling crate `starter-blob-garage`, which adds
  bucket lifecycle on top of this data plane)

## Quick start

```rust
use starter_blob_s3::{S3BlobStore, S3BlobStoreConfig};
use starter_spi::blob::BackendId;

let cfg = S3BlobStoreConfig::new(
    BackendId::new("s3:us-east-1:my-bucket"),
    "my-bucket",
    "us-east-1",
);
let store = S3BlobStore::open(cfg).await?;
```

Override the endpoint for MinIO / Garage / on-prem:

```rust
let cfg = S3BlobStoreConfig::new(
    BackendId::new("s3:garage:test"),
    "starter-test",
    "garage",
)
.endpoint_url("http://localhost:3900")
.force_path_style(true);
```

## Credentials

Two paths, both first-class:

| Constructor                         | Source of credentials                          |
| ----------------------------------- | ---------------------------------------------- |
| `S3BlobStore::open`                 | AWS SDK credential chain (env, files, IMDS…)   |
| `S3BlobStore::open_with_credentials`| Explicit `S3Credentials` (`SecretStore`-sourced)|

Garage deployments use the explicit path: the access-key is minted
per-bucket on Garage's admin API and persisted in a
`starter_spi::secrets::SecretStore`.

## Error mapping

| HTTP status / SDK event | `BlobError` variant                  |
| ----------------------- | ------------------------------------ |
| `404`                   | `NotFound` *(never collapsed onto Forbidden)* |
| `401`                   | `Unauthorized`                       |
| `403`                   | `Forbidden` *(never collapsed onto NotFound)* |
| `409`                   | `AlreadyExists`                      |
| `412`                   | `PreconditionFailed` (or `AlreadyExists` if `if_absent` set) |
| `413`                   | `PayloadTooLarge`                    |
| `429` / `503` (SlowDown) | `Throttled { retry_after }` parsed from `Retry-After` (capped at 300 s) |
| SDK `TimeoutError`      | `Timeout`                            |
| anything else           | `Backend(SdkErrorWrapper { op, .. })`|

The 403 ↔ 404 distinction is deliberately preserved: collapsing
them is the common lint failure that hides permission bugs as
"harmless misses."

## Multipart `put_stream`

`put_stream` buffers the first ~8 MiB to decide between a single
`PutObject` and a full multipart upload. Streams that fit in one
part avoid the round-trip overhead; larger streams chunk
transparently and abort the multipart upload on any per-part
failure.

## Conditional writes

`PutOptions::if_absent` maps to `If-None-Match: *`. The S3 server
returns `412 PreconditionFailed` on a hit, which this engine
re-maps to `BlobError::AlreadyExists` so the trait contract holds
(see the table above).

Finer-grained `if_match` against a specific ETag is on the
roadmap — the SPI `PutOptions` is `#[non_exhaustive]` so adding it
later is semver-additive across the engine crates.

## Licensing

`starter-blob-s3` is **MIT OR Apache-2.0**. The AWS SDK it links is
Apache-2.0. When the configured endpoint is a Garage cluster the
only contact with Garage is the S3 wire protocol — **no Garage Rust
crate is linked** from this crate or its transitive dependency
tree. Verify with:

```sh
cargo tree -p starter-blob-s3 | grep -i garage   # expected: empty
```

Garage itself is AGPL-3.0; the AGPL boundary stops at the network
seam.
