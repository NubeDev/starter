//! `BlobStore` — the seam over memory, fs, S3, Garage, and any
//! future object store. See the module-level docs in
//! [`super`](super) for the rules this trait is designed against
//! (B1 / B2 / B3 / R4 / R8).
//!
//! # Sync vs async
//!
//! Async, unlike [`crate::secrets::SecretStore`]. Every interesting
//! backend (S3, Garage, even the fs engine streaming a multi-GB
//! upload) is fundamentally async-shaped: a blocking trait would
//! force engines to spawn a runtime per call or block tokio worker
//! threads. The cost of an async-trait in the test engines
//! (memory, fs) is negligible — they `await` on already-resolved
//! futures.
//!
//! # Streams over `Vec<u8>`
//!
//! `put_stream` and `get` deliberately speak in
//! `Stream<Item = Result<Bytes, BlobError>>` rather than `Vec<u8>`
//! or `AsyncRead`. Two reasons:
//!
//! - `AsyncRead` is poorly served by `async-trait` (lifetime
//!   gymnastics, no `Send` by default), and forces engines into
//!   pin-projection rather than the natural
//!   `try_stream! { ... }` shape.
//! - `Stream<Bytes>` is what every backend SDK we wrap already
//!   speaks (`aws-sdk-s3` returns `ByteStream`, `axum` consumes
//!   `Body::data_stream`). Threading a different shape through
//!   would mean a copy at every engine boundary.

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use super::blob_ref::BlobRef;
use super::error::BlobError;
use super::key::BlobKey;
use super::meta::{BlobMeta, BlobRange};
use super::presigned::{PresignOp, PresignedUrl};

/// Caller-supplied hints for [`BlobStore::put_bytes`] /
/// [`BlobStore::put_stream`].
///
/// Optional fields throughout — engines that cannot honour a hint
/// drop it rather than failing. The fields chosen here are the
/// ones every shipped backend can store and round-trip on
/// [`BlobStore::head`]; backend-specific knobs (SSE config, S3
/// storage class) live on the engine's concrete type, not on this
/// SPI struct. That keeps the seam honest across backends and
/// lets engines surface their native config without polluting the
/// trait.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PutOptions {
    /// IANA media type to record alongside the bytes.
    pub content_type: Option<String>,

    /// `Cache-Control` directive to record alongside the bytes;
    /// returned by engines that serve presigned GETs.
    pub cache_control: Option<String>,

    /// If `Some`, fail with [`BlobError::AlreadyExists`] when the
    /// key already has bytes — `If-None-Match: *` semantics.
    pub if_absent: bool,
}

impl PutOptions {
    /// Build options with a content type set.
    pub fn with_content_type(content_type: impl Into<String>) -> Self {
        Self {
            content_type: Some(content_type.into()),
            ..Self::default()
        }
    }
}

/// One page of a [`BlobStore::list`] result.
///
/// Returns `(BlobRef, BlobMeta)` pairs — never raw `BlobKey`s.
/// Per the source SCOPE's B2 commentary, exposing raw keys here
/// would let consumers route around combinators and undo the
/// whole composition story. If you want to enumerate then read,
/// the natural shape is `list().items.into_iter().map(|(r, _)|
/// get(&r))`.
#[derive(Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ListPage {
    /// Items returned in this page. Order is engine-defined; do
    /// not rely on lexicographic sort except where the engine's
    /// docs promise it.
    pub items: Vec<(BlobRef, BlobMeta)>,

    /// Opaque cursor to pass back on the next call to
    /// [`BlobStore::list`] for the next page. `None` means
    /// "no more". Treat as a backend-defined token; do not
    /// inspect.
    pub next_cursor: Option<String>,
}

/// The blob-store trait.
///
/// Every method takes `&self` so engines can be wrapped in
/// `Arc<dyn BlobStore>` and shared across tasks; engines manage
/// any internal mutability themselves.
///
/// # Trait-object friendliness
///
/// The trait is intentionally `dyn`-compatible (object-safe): all
/// methods take `&self` and erased input/output shapes
/// (`BoxStream`, `Vec<_>`). Engine crates are expected to be
/// stored as `Arc<dyn BlobStore>` in consumer DI containers — the
/// `Arc` is the only way the SwapTest in the SCOPE works without a
/// generic explosion.
#[async_trait]
pub trait BlobStore: Send + Sync + 'static {
    /// Stable identifier the engine reports on every minted
    /// [`BlobRef`]. Useful for combinators that route on
    /// `backend_id`, and for operator-facing diagnostics.
    fn backend_id(&self) -> &super::blob_ref::BackendId;

    /// Store `bytes` under `key`. Returns the minted
    /// [`BlobRef`] which the consumer persists.
    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError>;

    /// Stream-style upload. Engines that support multipart upload
    /// (S3, Garage) chunk transparently; small-object engines
    /// (memory, fs) collect into a single write. Either way the
    /// returned `BlobRef` is durable on success.
    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError>;

    /// Stream the blob's bytes. `range` is honoured when `Some`;
    /// `None` means "the whole object". Range semantics mirror
    /// HTTP exactly — see [`BlobRange`].
    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError>;

    /// Fetch metadata only.
    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError>;

    /// Delete the blob. Idempotent — deleting a missing blob is
    /// `Ok(())`. Engines that genuinely cannot tell whether a
    /// delete succeeded (eventually-consistent backends mid-
    /// replication) should still return `Ok(())` once the
    /// delete tombstone is durable; the consumer should not
    /// observe an inconsistent "delete-then-get-still-finds-it"
    /// state from this trait.
    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError>;

    /// Enumerate blobs whose key starts with `prefix`.
    ///
    /// Returns `(BlobRef, BlobMeta)` pairs, never raw keys —
    /// see [`ListPage`].
    ///
    /// `cursor` is the opaque continuation token from a prior
    /// page; pass `None` on the first call.
    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError>;

    /// Mint a time-bounded URL granting `op` on `blob_ref`.
    ///
    /// Engines that cannot presign return
    /// [`BlobError::Unsupported`]; do not silently substitute a
    /// proxying route — that would change durability and latency
    /// posture without the consumer noticing.
    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError>;

    /// Server-side copy from `src` to `dst_key` on the same
    /// backend.
    ///
    /// Default impl returns [`BlobError::Unsupported`]. Engines
    /// that *can* honour the copy (S3, Garage — both support
    /// `CopyObject`) override; engines that cannot (memory, fs
    /// across mounts) leave the default and force the consumer to
    /// fall through to [`copy_via_client`]. **This is the B3
    /// surface**: the trait *requires* engines to refuse rather
    /// than silently read-then-write, so the consumer always
    /// knows when bytes traverse their process.
    async fn copy_server_side(
        &self,
        _src: &BlobRef,
        _dst_key: &BlobKey,
    ) -> Result<BlobRef, BlobError> {
        Err(BlobError::Unsupported)
    }
}

/// Cross-backend copy by streaming bytes through the caller's
/// process.
///
/// The B3-compliant fallback for
/// [`BlobStore::copy_server_side`]: when an engine refuses, the
/// consumer reaches for this helper *explicitly*, naming the
/// durability cost (bytes traverse the application) in the call
/// site. The function lives in `starter-spi` rather than in any
/// engine crate because it is engine-agnostic — it speaks only to
/// the trait — and a consumer should be able to reach for it
/// without taking a dep on a specific engine.
///
/// Streams the body from `src_store.get(src)` directly into
/// `dst_store.put_stream(dst_key, ...)`, preserving the
/// `content_type` reported by `src_store.head(src)` so the
/// destination blob looks identical on `head`. Other metadata
/// hints can be layered on by the caller before
/// `dst_store.head(new_ref)` if needed.
pub async fn copy_via_client(
    src_store: &dyn BlobStore,
    src: &BlobRef,
    dst_store: &dyn BlobStore,
    dst_key: &BlobKey,
) -> Result<BlobRef, BlobError> {
    let meta = src_store.head(src).await?;
    let stream = src_store.get(src, None).await?;
    let opts = PutOptions {
        content_type: meta.content_type,
        cache_control: meta.cache_control,
        if_absent: false,
    };
    dst_store.put_stream(dst_key, stream, opts).await
}
