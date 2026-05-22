//! Metadata projections — what `head` returns, what `put` accepts,
//! and how `get` asks for a byte range.
//!
//! # Why a separate `BlobMeta` rather than fields on `BlobRef`
//!
//! `BlobRef` is the *handle*; `BlobMeta` is the *observed state*.
//! They diverge: a `BlobRef` is stable for the lifetime of a
//! blob's identity, while `content_type` / `cache_control` /
//! `created_at` can change on overwrite. Threading them through
//! `BlobRef` would force callers that only need identity (combinator
//! routing) to carry fields they do not care about, and would force
//! a `BlobRef` rewrite on every metadata-only update.
//!
//! `BlobMeta` also deliberately does **not** carry a [`BlobKey`].
//! Exposing the original key here would re-introduce the B2
//! violation `BlobRef` was designed to prevent — a consumer could
//! call `head(blob_ref).key` and route around the store. The key
//! lives only on the input side of `put` / `list`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::blob_ref::Etag;

/// Observable metadata for a stored blob.
///
/// Returned from [`super::BlobStore::head`] and as the second
/// element of each pair from [`super::BlobStore::list`]. Engines
/// fill in what their backend supports; missing fields are `None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BlobMeta {
    /// Byte length of the stored content.
    pub size: u64,

    /// Backend-defined version marker, matching the
    /// [`super::BlobRef::etag`] returned at `put` time.
    pub etag: Etag,

    /// IANA media type (e.g. `image/png`). `None` when the engine
    /// has no opinion; the consumer is expected to fall back to a
    /// content-sniff or domain-level default rather than the
    /// engine guessing.
    pub content_type: Option<String>,

    /// `Cache-Control` directive to forward when the bytes are
    /// served via HTTP (presigned GET, or a downstream CDN). Stored
    /// alongside the blob — engines that cannot store free-form
    /// headers (the memory engine in its simplest form) round-trip
    /// what they were given on `put`.
    pub cache_control: Option<String>,

    /// First-write timestamp. Engines that cannot honestly report
    /// creation time (filesystems on some platforms) set this to
    /// `None` rather than guess.
    pub created_at: Option<DateTime<Utc>>,

    /// Last-write timestamp. Same engine-honesty caveat.
    pub updated_at: Option<DateTime<Utc>>,
}

impl BlobMeta {
    /// Construct a minimal [`BlobMeta`] carrying just the
    /// always-known fields (`size`, `etag`). Optional fields
    /// default to `None`; layer them on with the `with_*`
    /// setters.
    ///
    /// `BlobMeta` is `#[non_exhaustive]` so adding a future field
    /// (say, `checksum_sha256`) is semver-additive. Engines must
    /// not literal-construct the struct from outside this crate;
    /// they go through this constructor and the setters below so
    /// new fields land with sane defaults rather than breaking
    /// the build.
    pub fn new(size: u64, etag: Etag) -> Self {
        Self {
            size,
            etag,
            content_type: None,
            cache_control: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set [`BlobMeta::content_type`].
    pub fn with_content_type(mut self, content_type: Option<String>) -> Self {
        self.content_type = content_type;
        self
    }

    /// Set [`BlobMeta::cache_control`].
    pub fn with_cache_control(mut self, cache_control: Option<String>) -> Self {
        self.cache_control = cache_control;
        self
    }

    /// Set [`BlobMeta::created_at`].
    pub fn with_created_at(mut self, created_at: Option<DateTime<Utc>>) -> Self {
        self.created_at = created_at;
        self
    }

    /// Set [`BlobMeta::updated_at`].
    pub fn with_updated_at(mut self, updated_at: Option<DateTime<Utc>>) -> Self {
        self.updated_at = updated_at;
        self
    }
}

/// Inclusive byte range for a partial [`super::BlobStore::get`].
///
/// Semantics mirror HTTP `Range: bytes=start-end` exactly: both
/// endpoints are inclusive, `end` >= `start`, and `end` may
/// exceed the object size (engines clamp). Picking
/// HTTP-compatible semantics — rather than Rust-style
/// `start..end` half-open — means presigned GETs and direct
/// `get(Range)` calls return identical bytes for identical input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRange {
    /// First byte to return (inclusive, 0-indexed).
    pub start: u64,

    /// Last byte to return (inclusive). Use [`u64::MAX`] for
    /// "until end of object".
    pub end: u64,
}

impl BlobRange {
    /// Convenience constructor for `start..=end`. Returns `None`
    /// when `end < start`.
    pub fn new(start: u64, end: u64) -> Option<Self> {
        if end < start {
            None
        } else {
            Some(Self { start, end })
        }
    }

    /// Range covering `[start, ∞)`.
    pub fn from(start: u64) -> Self {
        Self {
            start,
            end: u64::MAX,
        }
    }
}
