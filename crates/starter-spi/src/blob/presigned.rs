//! `PresignedUrl` — a time-bounded URL the consumer can hand to a
//! browser or third-party process so the bytes flow without
//! traversing the application server.
//!
//! # Why this is a primitive, not a feature
//!
//! Presigning is the load-bearing reason this trait exists rather
//! than a hand-rolled `Vec<u8>` accessor in each consumer. Without
//! it the consumer becomes a byte-proxy for every download
//! (memory pressure, latency, billing). The SPI cannot make
//! presigning optional and still claim to abstract over real
//! object stores.
//!
//! Engines that genuinely cannot presign (a hypothetical embedded
//! backend) return [`super::BlobError::Unsupported`]; the test
//! engines (`-memory`, `-fs`) ship feature-gated axum routers that
//! honour their own presigned URLs precisely so the presign
//! contract is testable end-to-end without a live S3.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Which HTTP verb a [`PresignedUrl`] grants.
///
/// Kept as a small enum rather than a free `Method` because
/// `BlobStore` deliberately does not expose a generic
/// "presign anything" surface — only the two operations whose
/// semantics it can guarantee across backends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PresignOp {
    /// Download the blob's bytes.
    Get,

    /// Upload bytes for an existing [`super::BlobRef`]. The engine
    /// is responsible for binding the URL to a single key; the
    /// consumer cannot redirect the upload to a different blob.
    Put,
}

/// A time-bounded URL issued by [`super::BlobStore::presign`].
///
/// Carry it as-is to the client; do not parse or rewrite.
/// `expires_at` is the engine's commitment — the engine
/// guarantees the URL is valid until that instant and is free to
/// invalidate it sooner if backend policy demands (e.g. an S3 key
/// rotation). Consumers that need a stricter lifetime contract
/// should re-presign on demand rather than caching the URL past
/// its `expires_at`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresignedUrl {
    /// The URL to hand to the client.
    pub url: String,

    /// HTTP method the URL is signed for. The consumer must use
    /// this verb; the engine does not sign for `*`.
    pub method: PresignOp,

    /// Absolute expiry. Picked as [`SystemTime`] rather than a
    /// `Duration`-from-now so a `PresignedUrl` round-tripped
    /// through a queue or database does not silently extend.
    pub expires_at: SystemTime,
}
