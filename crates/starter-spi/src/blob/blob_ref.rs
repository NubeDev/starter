//! `BlobRef` — the opaque, time-crossing handle to a stored blob.
//!
//! # Why opaque
//!
//! Hard rule **B2** says `BlobRef` is the *only* handle that
//! crosses time (process boundary, DB column, JSON payload) and
//! must not let the consumer recover the raw key. If a consumer
//! could read the inner locator and route around the store, every
//! combinator in `starter-blob-compose` would become advisory
//! rather than load-bearing — `Namespaced` would not isolate
//! tenants, `Tiered` would not control promotion, `Mirrored` would
//! not guarantee write fan-out. The whole composition story
//! collapses.
//!
//! Compile-time enforcement, per the source SCOPE's Q2 resolution:
//!
//! - All fields are `pub(crate)`. The fields are reachable inside
//!   `starter-spi` and through the [`BlobRefInternal`] trait, but
//!   not by name from outside the crate.
//! - There is no `pub fn key()` / `pub fn locator()` accessor.
//! - The [`std::fmt::Debug`] impl prints `{ backend_id, etag,
//!   size, .. }` — it omits the locator on purpose so a stray
//!   `tracing::debug!("{:?}", blob_ref)` cannot leak the routing
//!   info.
//! - No [`std::fmt::Display`] impl. There is no canonical
//!   string form a consumer is meant to round-trip; the only
//!   sanctioned serialisation is the full `serde` shape, which an
//!   engine can decode but a consumer cannot meaningfully
//!   inspect.
//!
//! Serialisation is the one round-trip the consumer is allowed —
//! they persist the `BlobRef` as a JSON column and pass it back to
//! the same engine. The shape is stable across `serde_json` /
//! `bincode` / SQLite text columns.

use serde::{Deserialize, Serialize};

/// Stable identifier for an engine instance.
///
/// Picked by the engine constructor and embedded into every
/// [`BlobRef`] it mints. Combinators inspect this on read to route
/// to the right inner store. Two distinct instances of the same
/// engine type (e.g. two `starter-blob-fs` roots) **must** have
/// distinct `BackendId`s; otherwise a `BlobRef` becomes ambiguous
/// after a wiring change.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendId(String);

impl BackendId {
    /// Wrap an engine-supplied identifier.
    ///
    /// Engines pick a stable string at construction time —
    /// `"memory:dev"`, `"fs:/var/lib/starter/blobs"`,
    /// `"s3:eu-west-1:my-bucket"`. There is no validation here on
    /// purpose: engines know what shape they need.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the backend id as a string. Combinators use this for
    /// routing; it is not consumer-facing.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BackendId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Entity tag — backend-defined opaque version marker for a blob's
/// bytes.
///
/// Most engines map this to the underlying object store's ETag
/// (S3, Garage). The memory and fs engines mint their own. Used
/// for `If-Match` / `If-None-Match` preconditions on overwrite.
/// Treated as an opaque token; no parsing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Etag(String);

impl Etag {
    /// Wrap a backend-minted ETag string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Etag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The opaque, durable handle to a stored blob.
///
/// Mint with [`BlobRefInternal::mint`] from inside an engine crate.
/// Consumers receive `BlobRef` values from
/// [`super::BlobStore::put_bytes`] / `put_stream`, persist them
/// (typically as a JSON column), and pass them back to `get` /
/// `head` / `delete` / `presign`. The four accessors below are the
/// *only* sanctioned shape consumers see.
///
/// Note the absence of `Display`, `key()`, and `locator()` — see
/// the module docs for the B2 reasoning.
#[derive(Clone, Serialize, Deserialize)]
pub struct BlobRef {
    pub(crate) backend_id: BackendId,
    pub(crate) opaque_locator: String,
    pub(crate) etag: Etag,
    pub(crate) size: u64,
}

impl BlobRef {
    /// The engine instance that minted this ref. Combinators use
    /// it to route reads; consumers typically ignore it.
    pub fn backend_id(&self) -> &BackendId {
        &self.backend_id
    }

    /// Backend-defined version marker. Stable for the lifetime of
    /// the blob's bytes; changes on overwrite.
    pub fn etag(&self) -> &Etag {
        &self.etag
    }

    /// Size of the stored object in bytes. Reflects what the
    /// engine reported at `put` time; if a `Mirrored` combinator
    /// re-uploads to a peer with a different on-disk
    /// representation (compression), the size returned to the
    /// consumer is the *logical* byte count — what `get` will
    /// stream back — not any backend's storage footprint.
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl std::fmt::Debug for BlobRef {
    /// Deliberately omits `opaque_locator` so log lines cannot
    /// reconstruct the routing info. See B2 in the module docs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobRef")
            .field("backend_id", &self.backend_id)
            .field("etag", &self.etag)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Engine-facing helper trait. Engines (`starter-blob-*` crates)
/// import this to mint and unpack `BlobRef`s.
///
/// # Why not `pub` fields
///
/// If we made the fields `pub`, any consumer could read or
/// construct a `BlobRef` from raw parts and route around the
/// engine. Hiding the fields behind a trait whose methods take
/// `&self` keeps construction inside engine code (where the
/// invariants live) and reading limited to combinators that
/// *need* the locator to dispatch.
///
/// # Why a trait rather than `pub(crate)` constructors
///
/// Engines are downstream crates and cannot reach `pub(crate)`
/// methods on `BlobRef`. The trait gives them a sanctioned door
/// while still keeping consumers out — consumer code does not
/// import this trait, and even if it does the methods are honest
/// about what they expose.
pub trait BlobRefInternal: Sized {
    /// Construct a `BlobRef` from raw engine-owned parts.
    fn mint(backend_id: BackendId, opaque_locator: String, etag: Etag, size: u64) -> Self;

    /// Borrow the opaque locator. Combinators decode it to find
    /// the inner ref / inner key.
    fn opaque_locator(&self) -> &str;

    /// Replace the locator while preserving identity. Used by
    /// combinators that wrap an inner ref — `Namespaced` encodes
    /// the prefix length, `Tiered` encodes the current tier.
    fn with_locator(self, opaque_locator: String) -> Self;
}

impl BlobRefInternal for BlobRef {
    fn mint(backend_id: BackendId, opaque_locator: String, etag: Etag, size: u64) -> Self {
        Self {
            backend_id,
            opaque_locator,
            etag,
            size,
        }
    }

    fn opaque_locator(&self) -> &str {
        &self.opaque_locator
    }

    fn with_locator(mut self, opaque_locator: String) -> Self {
        self.opaque_locator = opaque_locator;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BlobRef {
        BlobRef::mint(
            BackendId::new("memory:test"),
            "loc-1".to_string(),
            Etag::new("abc"),
            42,
        )
    }

    #[test]
    fn accessors_expose_safe_fields_only() {
        let r = sample();
        assert_eq!(r.backend_id().as_str(), "memory:test");
        assert_eq!(r.etag().as_str(), "abc");
        assert_eq!(r.size(), 42);
    }

    #[test]
    fn debug_omits_locator() {
        let r = sample();
        let s = format!("{r:?}");
        assert!(!s.contains("loc-1"), "Debug must not leak locator: {s}");
        assert!(s.contains("memory:test"));
        assert!(s.contains("abc"));
    }

    #[test]
    fn serde_roundtrips_full_shape() {
        let r = sample();
        let j = serde_json::to_string(&r).unwrap();
        // Persisted shape carries the locator so the engine can
        // decode the ref on the next get.
        assert!(j.contains("loc-1"));
        let back: BlobRef = serde_json::from_str(&j).unwrap();
        assert_eq!(back.opaque_locator(), "loc-1");
        assert_eq!(back.size(), 42);
    }

    #[test]
    fn internal_with_locator_preserves_identity() {
        let r = sample().with_locator("loc-2".into());
        assert_eq!(r.opaque_locator(), "loc-2");
        assert_eq!(r.backend_id().as_str(), "memory:test");
        assert_eq!(r.etag().as_str(), "abc");
        assert_eq!(r.size(), 42);
    }
}
