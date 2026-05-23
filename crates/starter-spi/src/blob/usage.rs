//! [`BlobUsage`] — bytes-and-objects view of a key-prefix.
//!
//! Returned by [`super::BlobStore::approximate_usage`]. The word
//! "approximate" is in the name deliberately: in-process engines
//! (memory, fs) can answer authoritatively by walking the key space;
//! list-backed engines (S3, Garage) answer from `ListObjectsV2` /
//! inventory and may lag a recently-completed write by seconds or
//! minutes. Callers that need authoritative numbers for billing must
//! reconcile against the engine's native usage report, not this
//! trait method — `approximate` makes that posture impossible to
//! miss at the call site.
//!
//! The type is `#[non_exhaustive]` so engines that later learn to
//! report a third dimension (e.g. multipart parts in flight) can
//! grow the struct semver-additively.

use serde::{Deserialize, Serialize};

/// Bytes and object counts under a prefix.
///
/// Both fields are `u64` rather than `usize` so deployments where a
/// 32-bit consumer talks to a 64-bit store still see the same range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BlobUsage {
    /// Sum of stored byte sizes (the `size` field of each
    /// [`super::BlobMeta`]). Does not include sidecar metadata,
    /// multipart overhead, or backend replication factor.
    pub bytes: u64,
    /// Number of distinct keys under the prefix.
    pub objects: u64,
}

impl BlobUsage {
    /// Build a usage tally from its parts. Engines reach for this
    /// rather than a struct literal because `BlobUsage` is
    /// `#[non_exhaustive]`.
    pub fn new(bytes: u64, objects: u64) -> Self {
        Self { bytes, objects }
    }
}
