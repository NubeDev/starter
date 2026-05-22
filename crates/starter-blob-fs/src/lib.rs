//! # starter-blob-fs
//!
//! Filesystem-backed [`BlobStore`](starter_spi::blob::BlobStore).
//! Suitable for single-node dev boxes, single-tenant on-prem, and
//! integration tests that need real disk semantics (atomic writes,
//! O_EXCL conditional creates, persistent state across process
//! restarts).
//!
//! # Layout
//!
//! All blobs live under a single `root` directory passed to
//! [`FsBlobStore::open`]. Object keys map 1:1 to relative paths
//! under `root`; [`starter_spi::blob::BlobKey`] validation has
//! already refused `..`, leading-`/`, and NUL bytes by the time
//! the engine sees a key, so the directory layout is bounded.
//!
//! # Atomic writes
//!
//! Every `put_bytes` / `put_stream` writes to a `tempfile::NamedTempFile`
//! sibling and `persist`s into place. The intermediate file shares
//! the parent directory so the rename is atomic on POSIX. A
//! crash mid-write leaves the prior bytes intact — never a
//! half-written object.
//!
//! Conditional writes (`PutOptions::if_absent`) go through
//! [`OpenOptions::create_new`] (which maps to `O_EXCL` on POSIX);
//! the create races the rename, so a concurrent put with the same
//! key resolves to one winner and the loser surfaces
//! [`BlobError::AlreadyExists`].
//!
//! # Presign
//!
//! HMAC over a [`PresignKey`] supplied by the caller, typically
//! sourced from a [`SecretStore`](starter_spi::secrets::SecretStore).
//! The constructor [`FsBlobStore::open`] **requires** the key; it
//! never generates one implicitly because a key born inside the
//! engine on every restart would silently invalidate every
//! previously-issued URL — a B3-shaped durability shift.
//! [`PresignKey::ephemeral`] exists for tests with documented
//! die-with-process semantics.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod presign;
mod store;

#[cfg(feature = "axum")]
pub mod router;

pub use presign::{PresignClaim, PresignKey};
pub use store::{FsBlobStore, FsBlobStoreConfig, FsBlobStoreError};

/// Tracing target used by every span this engine emits.
pub(crate) const TRACE_TARGET: &str = "starter_blob::fs";
