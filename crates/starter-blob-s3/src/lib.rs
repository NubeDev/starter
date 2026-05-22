//! # starter-blob-s3
//!
//! S3-backed [`BlobStore`](starter_spi::blob::BlobStore). Wraps the
//! official [`aws-sdk-s3`] crate so the same engine talks to AWS S3,
//! MinIO, Cloudflare R2, Backblaze B2, Garage, and any other
//! S3-compatible endpoint. `force_path_style` is exposed on
//! [`S3BlobStoreConfig`] precisely because every non-AWS endpoint
//! needs it (DNS-style bucket addressing requires per-bucket TLS
//! certs, which on-prem deployments rarely have).
//!
//! # Credentials
//!
//! Two paths, both first-class — neither is a fallback for the other:
//!
//! - **SDK credential chain** (default). Build with
//!   [`S3BlobStore::open`]; the AWS SDK walks environment variables,
//!   shared config files, IMDS, and STS in the standard order.
//! - **Explicit `SecretStore`-sourced credentials**. Build with
//!   [`S3BlobStore::open_with_credentials`] passing access-key + secret
//!   pulled from a [`starter_spi::secrets::SecretStore`]. Garage
//!   deployments use this path so the credentials never touch
//!   `~/.aws/credentials`.
//!
//! # Hard rules
//!
//! - **B1 — no domain leakage**: the public surface is the
//!   [`BlobStore`](starter_spi::blob::BlobStore) trait. No bucket-
//!   or object-shaped methods leak out.
//! - **B3 — no silent durability shift**: 403, 404, and
//!   `SlowDown` map to distinct [`BlobError`](starter_spi::blob::BlobError)
//!   variants. We never collapse 403 onto 404 (would hide a
//!   permission bug behind a "harmless" miss), and we never collapse
//!   `SlowDown` onto a generic `Backend` (would silently break
//!   client-side backoff).
//!
//! # Licensing
//!
//! `starter-blob-s3` is **MIT OR Apache-2.0**. The AWS SDK it links
//! is Apache-2.0. When the configured endpoint is a Garage cluster
//! the only contact with Garage is the S3 wire protocol; no Garage
//! Rust crate is linked from this crate or its dependency tree —
//! verify with `cargo tree | grep garage` (should be empty). Garage
//! itself is AGPL-3.0 but starter consumers reach it over the wire
//! and the AGPL boundary stops at the network seam.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod store;

pub use error::map_sdk_error;
pub use store::{S3BlobStore, S3BlobStoreConfig, S3BlobStoreError, S3Credentials};

/// Tracing target every span this engine emits is routed under, per
/// the SPI module's observability contract.
pub(crate) const TRACE_TARGET: &str = "starter_blob::s3";
