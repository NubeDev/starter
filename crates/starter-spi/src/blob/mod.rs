//! Blob-store seam. Implementations live in engine crates
//! (`starter-blob-memory`, `starter-blob-fs`, `starter-blob-s3`,
//! `starter-blob-garage`) and combinators in `starter-blob-compose`;
//! this module defines the trait everything programs against.
//!
//! # Why blobs are a separate trait from secrets / sql
//!
//! Per `DOCS/storage/SCOPE.md` rule **R4** the workspace deliberately
//! refuses a universal `Store<K, V>` trait — relational stores
//! diverge too much across backends. `BlobStore` is only the
//! *second* trait that earned its place in `starter-spi` (the first
//! was [`SecretStore`](crate::secrets::SecretStore)) because the
//! blob contract — bytes-in, bytes-out, opaque handle — is wide
//! enough across object stores (memory, fs, S3, Garage) that one
//! trait is honestly portable. Anything narrower would re-introduce
//! domain leakage; anything wider would not survive the first real
//! backend swap.
//!
//! # Hard rules (B1, B2, B3) and where they bite
//!
//! - **B1 — no domain leakage.** [`BlobStore`] knows only about
//!   `BlobKey` / `BlobRef` / bytes / metadata. It must never grow a
//!   `put_avatar` or `get_attachment`-shaped method. Domain
//!   repositories live in consumer code and *call* a `BlobStore`,
//!   not the other way around.
//! - **B2 — `BlobRef` is opaque.** The struct has no `pub fn key()`,
//!   no `Display` impl, and its locator field is `pub(crate)` —
//!   reachable only through the engine-facing
//!   [`BlobRefInternal`](self::blob_ref::BlobRefInternal) helper
//!   trait. A consumer cannot, by construction, recover the raw key
//!   from a `BlobRef`. That makes B2 a compile-time fact rather
//!   than a code-review guideline. See
//!   [`BlobRef`](self::blob_ref::BlobRef) for the freeze.
//! - **B3 — no silent durability shift.** Engines surface
//!   [`BlobError::Unsupported`] from operations they cannot honour
//!   (e.g. `copy_server_side` across distinct backends). They never
//!   silently fall back to a client-side path that changes the
//!   durability story. Combinators in `starter-blob-compose` name
//!   their durability mode in their *type name* (e.g.
//!   `Mirrored::AsyncBackground`); this trait surface gives them no
//!   room to hide.
//!
//! # Observability contract
//!
//! This module — and every other module under `starter-spi` —
//! emits **no** `tracing` spans and **no** metrics. The
//! observability contract puts emission in the engine crates under
//! the `starter_blob::<engine>` tracing target (e.g.
//! `starter_blob::memory`, `starter_blob::s3`). Metrics are opt-in
//! per engine and registered against the consumer's
//! `prometheus::Registry` exactly the way `Service` does it
//! elsewhere in this crate. The reasoning: a span emitted from
//! `spi` would force every consumer to filter it out, and would
//! lie about who actually did the I/O (the engine is the
//! interesting level of granularity for an operator).
//!
//! # Module layout
//!
//! Per `HOW-TO-ADD-CODE.md` rule "one responsibility per file" the
//! body of every public item lives in its own file. This file is a
//! re-export barrel.

mod blob_ref;
mod context;
mod error;
mod key;
mod meta;
pub mod meta_keys;
mod presigned;
mod store;

pub use blob_ref::{BackendId, BlobRef, BlobRefInternal, Etag};
pub use context::BlobContext;
pub use error::{BlobError, BoxError};
pub use key::{BlobKey, BlobKeyError, MAX_BLOB_KEY_LEN};
pub use meta::{BlobMeta, BlobRange};
pub use presigned::{PresignOp, PresignedUrl};
pub use store::{copy_via_client, BlobStore, ListPage, PutOptions};
