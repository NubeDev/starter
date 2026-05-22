//! # starter-blob-garage
//!
//! Garage-aware [`BlobStore`](starter_spi::blob::BlobStore). The
//! data plane delegates to [`starter_blob_s3::S3BlobStore`] over
//! Garage's S3-compatible endpoint; this crate layers Garage-
//! specific lifecycle on top:
//!
//! - bucket lifecycle via the admin API (`POST /v1/bucket`,
//!   `DELETE /v1/bucket/{id}`),
//! - per-bucket access-key minting (`POST /v1/key`,
//!   `POST /v1/bucket/allow`),
//! - cluster health (`GET /v1/health`) surfaced as a typed status,
//! - cluster-layout introspection at startup (`GET /v1/layout`).
//!
//! # Why a separate crate
//!
//! A consumer that only needs S3-shaped access reaches for
//! `starter-blob-s3` directly — no Garage admin surface, no
//! reqwest dependency. Consumers that operate a Garage cluster get
//! the full lifecycle helpers here. The trait surface is unchanged:
//! a `GarageBlobStore` is a `BlobStore`, and the admin client is a
//! sibling type the consumer reaches for explicitly.
//!
//! # Licensing
//!
//! `starter-blob-garage` is **MIT OR Apache-2.0**. Garage itself is
//! AGPL-3.0; this crate reaches Garage only over its HTTP S3 and
//! admin APIs. **No Garage Rust crate is linked** from this crate's
//! dependency tree — verify with `cargo tree -p starter-blob-garage
//! | grep -i garage` (should match only this crate's own name).
//! The AGPL boundary is the network, not the source tree, so
//! starter-side consumers do not inherit AGPL obligations.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod admin;
mod store;

pub use admin::{
    BucketInfo, ClusterHealth, ClusterStatus, GarageAdmin, GarageAdminError, GarageKey, LayoutInfo,
};
pub use store::{GarageBlobStore, GarageBlobStoreError};

/// Re-export so consumers do not have to take a direct dep on
/// `starter-blob-s3` to construct the data-plane config.
pub use starter_blob_s3::{S3BlobStoreConfig, S3Credentials};
