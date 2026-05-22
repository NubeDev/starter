//! `BlobKey` — a validated, UTF-8, length-bounded key used by
//! consumers to address blobs *before* a [`BlobRef`] exists.
//!
//! # Why a newtype rather than `String`
//!
//! Engines (fs, S3, Garage) all share a small set of failure modes
//! that come from accepting raw strings: directory traversal via
//! `..`, absolute-path collisions via a leading `/`, NUL bytes that
//! some backends silently truncate, and length bombs that exceed
//! S3's 1024-byte object-key cap. Validating once at the type
//! boundary means every engine can `&BlobKey` without re-checking,
//! and the failure mode is a single typed
//! [`BlobKeyError`] rather than a different per-engine error per
//! backend.
//!
//! # Why this lives on the seam
//!
//! `BlobKey` is *not* opaque the way [`super::BlobRef`] is — the
//! consumer chose the key in the first place, so hiding it would
//! be theatre. It is the *handle that crosses time* —
//! [`super::BlobRef`] — that B2 makes opaque. Keys are first-class
//! input.

use serde::{Deserialize, Serialize};

/// Upper bound on a serialised `BlobKey` in bytes. Picked to match
/// the S3 object-key limit (1024 bytes UTF-8) so the SPI does not
/// silently accept keys that the production engine will reject.
/// The same limit comfortably covers the fs and memory engines.
pub const MAX_BLOB_KEY_LEN: usize = 1024;

/// A validated blob key.
///
/// Construct via [`BlobKey::new`]; the only way to obtain one is to
/// pass validation, so engines that take `&BlobKey` never have to
/// re-check. Keys serialise as plain strings — `serde(transparent)`
/// — so a SQLite/JSON column round-trips verbatim, but
/// deserialisation re-runs validation so a tampered store cannot
/// smuggle a `..` past the boundary.
#[derive(Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct BlobKey(String);

impl BlobKey {
    /// Validate and wrap.
    ///
    /// Rules — each one corresponds to a failure mode an engine
    /// would otherwise hit:
    /// - non-empty (S3 rejects, fs would map to a directory),
    /// - valid UTF-8 (enforced by `String`; this constructor takes
    ///   `impl Into<String>` so the caller has already passed that
    ///   gate),
    /// - length ≤ [`MAX_BLOB_KEY_LEN`] bytes,
    /// - no leading `/` (would collide with absolute-path semantics
    ///   on the fs engine and is a common path-traversal vector),
    /// - no `..` path segment (directory traversal on fs; on S3 the
    ///   key is technically legal but downstream UNIX tooling
    ///   trips),
    /// - no embedded NUL byte (some backends silently truncate).
    pub fn new(value: impl Into<String>) -> Result<Self, BlobKeyError> {
        let value: String = value.into();
        if value.is_empty() {
            return Err(BlobKeyError::Empty);
        }
        if value.len() > MAX_BLOB_KEY_LEN {
            return Err(BlobKeyError::TooLong {
                len: value.len(),
                max: MAX_BLOB_KEY_LEN,
            });
        }
        if value.starts_with('/') {
            return Err(BlobKeyError::LeadingSlash);
        }
        if value.contains('\0') {
            return Err(BlobKeyError::NulByte);
        }
        for segment in value.split('/') {
            if segment == ".." {
                return Err(BlobKeyError::ParentSegment);
            }
        }
        Ok(Self(value))
    }

    /// Borrow the underlying string. Engines need this to build a
    /// backend-native object key; it is not a B2 violation because
    /// `BlobKey` is the consumer-supplied input, not the
    /// time-crossing handle.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the key and return the inner `String`. Useful for
    /// engines that want to push the key into a `format!` without
    /// an extra allocation.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for BlobKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlobKey({:?})", self.0)
    }
}

impl std::fmt::Display for BlobKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BlobKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        BlobKey::new(raw).map_err(serde::de::Error::custom)
    }
}

/// Reasons a [`BlobKey`] constructor refuses an input. Each variant
/// names the specific rule violated so consumers can render a
/// useful operator-facing error rather than a generic "invalid
/// key".
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlobKeyError {
    /// Key was the empty string.
    #[error("blob key must not be empty")]
    Empty,

    /// Key exceeded [`MAX_BLOB_KEY_LEN`] bytes.
    #[error("blob key length {len} exceeds max {max}")]
    TooLong {
        /// Length of the rejected key, in bytes.
        len: usize,
        /// Configured maximum.
        max: usize,
    },

    /// Key began with `/`.
    #[error("blob key must not start with '/'")]
    LeadingSlash,

    /// Key contained a `..` path segment.
    #[error("blob key must not contain a '..' segment")]
    ParentSegment,

    /// Key contained an embedded NUL byte.
    #[error("blob key must not contain a NUL byte")]
    NulByte,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(BlobKey::new("").unwrap_err(), BlobKeyError::Empty);
    }

    #[test]
    fn rejects_leading_slash() {
        assert_eq!(
            BlobKey::new("/abs/path").unwrap_err(),
            BlobKeyError::LeadingSlash
        );
    }

    #[test]
    fn rejects_parent_segment() {
        assert_eq!(
            BlobKey::new("a/../b").unwrap_err(),
            BlobKeyError::ParentSegment
        );
    }

    #[test]
    fn rejects_nul() {
        assert_eq!(BlobKey::new("a\0b").unwrap_err(), BlobKeyError::NulByte);
    }

    #[test]
    fn rejects_overlong() {
        let big = "a".repeat(MAX_BLOB_KEY_LEN + 1);
        match BlobKey::new(big).unwrap_err() {
            BlobKeyError::TooLong { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn accepts_normal_keys() {
        BlobKey::new("foo/bar.txt").unwrap();
        BlobKey::new("a").unwrap();
        BlobKey::new("tenants/7/avatars/me.png").unwrap();
    }

    #[test]
    fn dotdot_in_filename_ok() {
        // Only the segment `..` is forbidden; `..foo` is a legal
        // filename in every backend we ship.
        BlobKey::new("foo/..bar").unwrap();
    }

    #[test]
    fn serde_roundtrip_revalidates() {
        let k = BlobKey::new("foo/bar").unwrap();
        let s = serde_json::to_string(&k).unwrap();
        let back: BlobKey = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back);

        // A tampered store cannot smuggle a `..` through.
        let bad = serde_json::from_str::<BlobKey>("\"a/../b\"").unwrap_err();
        assert!(bad.to_string().contains(".."));
    }
}
