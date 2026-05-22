//! # starter-blob-memory
//!
//! In-process [`BlobStore`](starter_spi::blob::BlobStore) backed by a
//! `HashMap<BlobKey, Bytes>` behind an `RwLock`. Use cases:
//!
//! - **Tests.** The SCOPE's `TestWithoutNetwork` smoke fixture runs
//!   the full integration suite against this engine with no
//!   feature-flag gymnastics; no S3, no fs, no network.
//! - **Dev loops.** Spin a `MemoryBlobStore`, hand it to consumer
//!   code, throw away on shutdown.
//!
//! # Durability
//!
//! There is none. Bytes die with the process. Surface this in any
//! consumer code path that *might* end up wired against the memory
//! engine in production by mistake.
//!
//! # Presign contract
//!
//! `presign` returns a URL pointed at the engine's feature-gated
//! axum [`router`] (compile with `--features axum`). The token is an
//! HMAC over `(op, locator, expires_at)` keyed by a **process-local**
//! random secret minted at engine construction time. Two
//! consequences fall out of "process-local":
//!
//! - Presigned URLs do not survive restart. After the process
//!   recycles, every previously-handed-out URL is `403`. That mirrors
//!   the real S3 behaviour where credentials rotate; tests rely on
//!   it.
//! - Two `MemoryBlobStore` instances inside the same process do
//!   **not** honour each other's URLs. The router is bound to a
//!   single store instance.
//!
//! The HMAC key is never logged, never serialised, never reachable
//! outside the engine. See [`presign`] for the wire shape.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod presign;
mod store;

#[cfg(feature = "axum")]
pub mod router;

pub use presign::PresignClaim;
pub use store::{MemoryBlobStore, MemoryBlobStoreConfig};

/// Tracing target every span in this engine emits under, per the
/// observability contract in `starter_spi::blob`.
pub(crate) const TRACE_TARGET: &str = "starter_blob::memory";
