//! Reserved user-metadata keys.
//!
//! [`BlobMeta::user_metadata`](super::BlobMeta::user_metadata) is a
//! free-form `String → String` map every engine round-trips
//! unchanged. To make a [`BlobRef`](super::BlobRef) portable between
//! consumers — and to give [`starter-blob-axum`]'s proxy handler a
//! stable place to read the original filename for
//! `Content-Disposition` — the three keys below are *reserved*:
//! every starter consumer agrees on their spelling and meaning.
//!
//! Consumers may add their own keys freely; only collisions with
//! these reserved names are prohibited.
//!
//! [`starter-blob-axum`]: https://docs.rs/starter-blob-axum
//!
//! # Why constants rather than free-form strings
//!
//! If each consumer wrote `meta.insert("filename", …)` directly, a
//! typo (`"file_name"`, `"FileName"`) would silently break the
//! round-trip. Exposing the keys as `pub const &str` makes the call
//! site grep-able and gives the compiler a chance to catch a rename
//! the day starter changes the spelling.

/// Original client-supplied filename, UTF-8.
///
/// Set by the upload pipeline at `put_*` time. Read by
/// `starter-blob-axum`'s proxy handler to populate
/// `Content-Disposition: attachment; filename="…"` when the client
/// requests a download.
pub const FILENAME: &str = "filename";

/// Opaque consumer-defined principal id that performed the upload.
///
/// Format is consumer-defined (user UUID, OAuth subject, service
/// account name). Engines do not interpret it; it is round-tripped
/// for audit trails and downstream authz decisions.
pub const UPLOADED_BY: &str = "uploaded_by";

/// RFC3339 timestamp of the upload.
///
/// Distinct from [`BlobMeta::created_at`](super::BlobMeta::created_at),
/// which is the engine's view of first-write time and may be `None`
/// on engines that cannot honestly report it. `UPLOADED_AT` is the
/// *consumer's* timestamp at the moment the upload was accepted,
/// and is always populated when the upload pipeline sets it.
pub const UPLOADED_AT: &str = "uploaded_at";

/// All reserved keys, in declaration order. Useful for validation
/// helpers that need to reject consumer-supplied keys colliding
/// with the reserved set.
pub const RESERVED: &[&str] = &[FILENAME, UPLOADED_BY, UPLOADED_AT];
