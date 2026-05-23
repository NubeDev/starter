//! # starter-blob-compose
//!
//! Combinators for [`BlobStore`](starter_spi::blob::BlobStore). Each
//! combinator *is itself* a `BlobStore`, so they nest without
//! consumer-side ceremony: a `Mirrored<Namespaced<Tiered<...>>, ...>`
//! is one `Arc<dyn BlobStore>` to the caller.
//!
//! # The four combinators, and why they are four and not three
//!
//! - [`Namespaced`] prepends a fixed prefix on every operation and
//!   strips it from `list()` results before yielding. Tenancy and
//!   per-feature key isolation without leaking domain words into
//!   the engine (B1).
//! - [`Tiered`] writes to a hot store, optionally demotes to a cold
//!   store per [`TieredPolicy`], and reads with hot-then-cold
//!   fallback. The hot/cold distinction is durability-relevant, so
//!   it appears in the type name (B3).
//! - [`Mirrored`] writes to a primary plus one or more mirrors.
//!   [`MirrorMode::Sync`] fails the put on any mirror failure;
//!   [`MirrorMode::AsyncBackground`] returns on primary success
//!   and best-effort fans out the mirrors. The mode load-bears B3
//!   — its name spells out the durability promise.
//! - [`ReadThroughCache`] writes only to the source and lazily
//!   populates a cache on read. Kept **separate** from `Tiered`
//!   because the write target differs: a `Tiered` write goes to
//!   hot first; a `ReadThroughCache` write goes to source only.
//!   Folding them together would force a `WriteTarget` enum that
//!   no consumer needs today.
//!
//! # `BlobRef` rewriting
//!
//! Every combinator re-mints `BlobRef`s on the way out. The outer
//! ref carries the combinator's own [`BackendId`] and an
//! `opaque_locator` that encodes whatever the combinator needs to
//! route a follow-up `get`/`head`/`delete`. Consumers persist the
//! outer ref verbatim; the combinator decodes on read. See the
//! per-combinator rustdoc for the locator shape.
//!
//! `list()` returns `(BlobRef, BlobMeta)` pairs — the same shape as
//! the underlying trait. Combinators rewrite the `BlobRef`s the
//! same way `put` does; the consumer's view stays consistent.
//!
//! # What is **not** here
//!
//! No `list_keys() -> Vec<String>`. It is deliberately not on the
//! trait (per the SCOPE) and combinators cannot opt in. Returning
//! raw strings would let a consumer route around the wrapper and
//! undo B2 in one line.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod codec;
mod mirrored;
mod namespaced;
mod read_through_cache;
mod tiered;

pub use mirrored::{MirrorMode, Mirrored, MirroredBuilder};
pub use namespaced::{Namespaced, Quota};
pub use read_through_cache::ReadThroughCache;
pub use tiered::{Tiered, TieredPolicy};

/// Tracing target every span in this crate emits under, per the
/// observability contract in [`starter_spi::blob`].
pub(crate) const TRACE_TARGET: &str = "starter_blob::compose";
