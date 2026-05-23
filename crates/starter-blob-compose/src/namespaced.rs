//! `Namespaced` — prefix-isolating combinator.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use starter_spi::blob::{
    BackendId, BlobContext, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal,
    BlobStore, BlobUsage, ListPage, PresignOp, PresignedUrl, PutOptions,
};

use crate::codec;

/// Wraps a [`BlobStore`] in a fixed key prefix.
///
/// Every key the caller supplies is prepended with `self.prefix`
/// before reaching the inner store; every key the inner store
/// reports back through `list()` has the prefix stripped before
/// reaching the caller. The caller observes a clean view rooted at
/// their namespace — they never see the prefix, and they cannot
/// escape it (no `..`, no leading `/`, both already barred by
/// [`BlobKey`]).
///
/// # Locator shape
///
/// `BlobRef::opaque_locator` is a JSON envelope `{"v":1,"payload":
/// inner_ref}` where `inner_ref` is the serialised form of the
/// inner store's [`BlobRef`]. `Namespaced` decodes this on every
/// subsequent operation so the inner store sees its own ref, not a
/// rewritten one.
///
/// # Failure modes (B3 — durability does not shift)
///
/// `Namespaced` is a pure routing layer. It does not change the
/// inner store's durability semantics, retry policy, or
/// consistency model. Every [`BlobError`] surfaced by the inner
/// store passes through unchanged. The only failure `Namespaced`
/// adds on its own is [`BlobError::backend`] when a caller-supplied
/// key would produce an invalid inner key after prefixing (length
/// over [`starter_spi::blob::MAX_BLOB_KEY_LEN`], or a structural
/// rule violated by the concatenation).
pub struct Namespaced {
    inner: Arc<dyn BlobStore>,
    prefix: String,
    backend_id: BackendId,
    quota: Option<Quota>,
}

/// Cap on bytes and/or object count under a [`Namespaced`] prefix.
///
/// `None` on a field means "no limit for this dimension". Either or
/// both may be set — `Quota { max_bytes: Some(1 << 30), max_objects:
/// None }` is a 1 GiB byte cap with no object-count cap.
///
/// # Where the counter lives
///
/// `Namespaced` deliberately keeps **no** in-memory counter. On
/// every write it asks the inner store via
/// [`BlobStore::approximate_usage`] for the current namespace
/// footprint and compares against the cap. The scope locks this in:
/// one source of truth per deployment, no drift between a
/// combinator's stale tally and the engine's authoritative one.
///
/// # Race window
///
/// The check is pre-flight: read usage, then put. Two concurrent
/// writers can both pass the check and both succeed, overshooting
/// the cap. This is intentional — closing that window would force
/// a global lock against the inner store, which is the wrong cost
/// shape for the use case (a noisy-neighbour deterrent, not a
/// hard-billing gate). Document the over-by-one-write behaviour at
/// the consumer level.
///
/// # Streaming caveat
///
/// `put_stream` does not know the body length up-front, so the
/// pre-flight check can only refuse a write into a namespace that
/// is *already* over the byte cap. A streamed write whose body
/// crosses the cap mid-stream is admitted; closing this gap means
/// counting bytes during the stream and aborting the inner put,
/// which is engine-coupled work. Tracked separately.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct Quota {
    /// Maximum total bytes under the namespace prefix.
    pub max_bytes: Option<u64>,
    /// Maximum object count under the namespace prefix.
    pub max_objects: Option<u64>,
}

impl Quota {
    /// Build a quota with a byte cap.
    pub fn max_bytes(bytes: u64) -> Self {
        Self {
            max_bytes: Some(bytes),
            max_objects: None,
        }
    }

    /// Build a quota with an object-count cap.
    pub fn max_objects(objects: u64) -> Self {
        Self {
            max_bytes: None,
            max_objects: Some(objects),
        }
    }

    /// Chain a byte cap onto an existing quota.
    pub fn with_max_bytes(mut self, bytes: u64) -> Self {
        self.max_bytes = Some(bytes);
        self
    }

    /// Chain an object-count cap onto an existing quota.
    pub fn with_max_objects(mut self, objects: u64) -> Self {
        self.max_objects = Some(objects);
        self
    }
}

#[derive(Serialize, Deserialize)]
struct Locator {
    inner: BlobRef,
}

impl Namespaced {
    /// Build a `Namespaced` over `inner` with the given key
    /// `prefix`.
    ///
    /// `prefix` must validate as a [`BlobKey`] — same rules apply
    /// to a namespace root as to any key (no `..`, no leading `/`,
    /// no NUL, length bounded). The caller is responsible for the
    /// trailing `/` if they want directory-style separation;
    /// `Namespaced` does not insert one because some namespaces
    /// legitimately want a non-`/` separator (e.g. `tenant-7-`).
    pub fn new(inner: Arc<dyn BlobStore>, prefix: impl Into<String>) -> Result<Self, BlobError> {
        let prefix: String = prefix.into();
        // Validate the prefix as a BlobKey to inherit the
        // length / structural rules. The BlobKey itself is
        // discarded — we only keep the string for concatenation.
        let _ = BlobKey::new(prefix.clone()).map_err(BlobError::backend)?;
        let backend_id = BackendId::new(format!("namespaced({})", inner.backend_id().as_str()));
        Ok(Self {
            inner,
            prefix,
            backend_id,
            quota: None,
        })
    }

    /// Attach a [`Quota`] to this namespace. Writes that would
    /// breach the cap fail with [`BlobError::PayloadTooLarge`].
    ///
    /// Requires the inner store to implement
    /// [`BlobStore::approximate_usage`] — otherwise the pre-flight
    /// check returns [`BlobError::Unsupported`] and `put_*` will
    /// propagate it. Engines (memory, fs, s3, garage) implement it;
    /// a pass-through combinator without an answer would, by
    /// construction, be unable to enforce a cap.
    pub fn with_quota(mut self, quota: Quota) -> Self {
        self.quota = Some(quota);
        self
    }

    /// The prefix this combinator prepends. Operator-facing
    /// diagnostic only — not part of the consumer surface.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Current quota, if any. Operator-facing diagnostic.
    pub fn quota(&self) -> Option<Quota> {
        self.quota
    }

    /// Pre-flight check against the inner store's
    /// [`BlobStore::approximate_usage`]. `incoming_bytes` is the
    /// known length of a `put_bytes` body; pass `None` for
    /// `put_stream` (we cannot count what we have not yet seen).
    /// `incoming_objects` is `1` when the write would create a new
    /// key, `0` for an overwrite — the caller cannot cheaply tell
    /// the two apart without a round-trip, so we conservatively
    /// pass `1` and accept that overwrites count as a fresh object
    /// in the pre-flight (the cap is still respected; only the
    /// rejection threshold is one object stricter than it could
    /// be).
    async fn check_quota(
        &self,
        incoming_bytes: Option<u64>,
        incoming_objects: u64,
    ) -> Result<(), BlobError> {
        let Some(q) = self.quota else {
            return Ok(());
        };
        if q.max_bytes.is_none() && q.max_objects.is_none() {
            return Ok(());
        }
        // The inner store sees its own keyspace — query under the
        // prefix that points at our namespace inside it.
        let prefix_key = BlobKey::new(self.prefix.clone()).map_err(BlobError::backend)?;
        let usage: BlobUsage = self.inner.approximate_usage(&prefix_key).await?;
        if let Some(cap) = q.max_bytes {
            let projected = match incoming_bytes {
                Some(n) => usage.bytes.saturating_add(n),
                None => usage.bytes,
            };
            if projected > cap {
                return Err(BlobError::PayloadTooLarge);
            }
        }
        if let Some(cap) = q.max_objects {
            let projected = usage.objects.saturating_add(incoming_objects);
            if projected > cap {
                return Err(BlobError::PayloadTooLarge);
            }
        }
        Ok(())
    }

    fn combined(&self, key: &BlobKey) -> Result<BlobKey, BlobError> {
        let mut combined = String::with_capacity(self.prefix.len() + key.as_str().len());
        combined.push_str(&self.prefix);
        combined.push_str(key.as_str());
        BlobKey::new(combined).map_err(BlobError::backend)
    }

    fn wrap(&self, inner_ref: BlobRef) -> BlobRef {
        let size = inner_ref.size();
        let etag = inner_ref.etag().clone();
        let locator = codec::encode(Locator { inner: inner_ref });
        BlobRef::mint(self.backend_id.clone(), locator, etag, size)
    }

    fn unwrap_ref(&self, outer: &BlobRef) -> Result<BlobRef, BlobError> {
        let loc: Locator = codec::decode(outer.opaque_locator())?;
        Ok(loc.inner)
    }
}

#[async_trait]
impl BlobStore for Namespaced {
    fn backend_id(&self) -> &BackendId {
        &self.backend_id
    }

    fn context_for(&self, blob_ref: &BlobRef) -> BlobContext {
        // Unwrap to the inner ref; if decoding fails (e.g. a
        // caller passed us a ref minted by a different combinator
        // stack), fall back to a context carrying just our prefix
        // rather than panicking — the proxy handler will reject
        // the request downstream when `get`/`head` returns an
        // error.
        match self.unwrap_ref(blob_ref) {
            Ok(inner) => self
                .inner
                .context_for(&inner)
                .prepend_namespace(self.prefix.clone()),
            Err(_) => BlobContext::empty()
                .with_backend_id(self.backend_id.clone())
                .prepend_namespace(self.prefix.clone()),
        }
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        self.check_quota(Some(bytes.len() as u64), 1).await?;
        let inner_key = self.combined(key)?;
        let inner_ref = self.inner.put_bytes(&inner_key, bytes, opts).await?;
        Ok(self.wrap(inner_ref))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // Pre-flight against the cap without a known body length —
        // we can only refuse a namespace that is *already* over.
        // See the `Quota` docstring on the streaming caveat.
        self.check_quota(None, 1).await?;
        let inner_key = self.combined(key)?;
        let inner_ref = self.inner.put_stream(&inner_key, stream, opts).await?;
        Ok(self.wrap(inner_ref))
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let inner = self.unwrap_ref(blob_ref)?;
        self.inner.get(&inner, range).await
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let inner = self.unwrap_ref(blob_ref)?;
        self.inner.head(&inner).await
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let inner = self.unwrap_ref(blob_ref)?;
        self.inner.delete(&inner).await
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        // Combine self.prefix + caller-supplied prefix to address
        // the inner store. Note we don't bounce through BlobKey
        // here when the caller-prefix is None and self.prefix is
        // empty — that would reject "" which we want to allow.
        let combined_prefix: Option<BlobKey> = match prefix {
            Some(p) => Some(self.combined(p)?),
            None if self.prefix.is_empty() => None,
            None => Some(BlobKey::new(self.prefix.clone()).map_err(BlobError::backend)?),
        };
        let page = self.inner.list(combined_prefix.as_ref(), cursor).await?;
        // Rewrap every inner BlobRef; cursor is left untouched
        // because it is the inner store's continuation token and
        // we must round-trip it verbatim. The consumer treats it
        // as opaque (per the trait docs).
        let items = page
            .items
            .into_iter()
            .map(|(r, m)| (self.wrap(r), m))
            .collect();
        Ok(ListPage::new(items, page.next_cursor))
    }

    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        let inner = self.unwrap_ref(blob_ref)?;
        self.inner.presign(&inner, op, ttl).await
    }

    async fn copy_server_side(
        &self,
        src: &BlobRef,
        dst_key: &BlobKey,
    ) -> Result<BlobRef, BlobError> {
        // A server-side copy adds a new object — count its size
        // against the cap. `head` against the inner ref tells us
        // how big.
        let inner_src = self.unwrap_ref(src)?;
        let incoming = self.inner.head(&inner_src).await?.size;
        self.check_quota(Some(incoming), 1).await?;
        let inner_dst = self.combined(dst_key)?;
        let inner_ref = self.inner.copy_server_side(&inner_src, &inner_dst).await?;
        Ok(self.wrap(inner_ref))
    }

    async fn approximate_usage(&self, prefix: &BlobKey) -> Result<BlobUsage, BlobError> {
        // Combine our prefix with the caller-supplied one before
        // asking the inner store — same shape as `list`.
        let combined = self.combined(prefix)?;
        self.inner.approximate_usage(&combined).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use starter_blob_memory::MemoryBlobStore;

    fn k(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    async fn drain(s: BoxStream<'static, Result<Bytes, BlobError>>) -> Vec<u8> {
        let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
        chunks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    fn store() -> (Arc<MemoryBlobStore>, Namespaced) {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner.clone(), "tenant-7/").unwrap();
        (inner, ns)
    }

    #[tokio::test]
    async fn put_prefixes_inner_key() {
        let (inner, ns) = store();
        let _r = ns
            .put_bytes(
                &k("avatar.png"),
                Bytes::from_static(b"x"),
                PutOptions::default(),
            )
            .await
            .unwrap();
        // The inner store sees the combined key.
        let page = inner.list(None, None).await.unwrap();
        let inner_keys: Vec<_> = page
            .items
            .iter()
            .map(|(ir, _)| ir.opaque_locator().to_owned())
            .collect();
        assert_eq!(inner_keys, vec!["tenant-7/avatar.png"]);
        // B2 holds at the *type* level: there is no public `key()`
        // accessor on the outer BlobRef. The locator string may
        // still embed the key text — that is fine because no
        // public API exposes it.
    }

    #[tokio::test]
    async fn get_head_delete_round_trip_through_wrapper() {
        let (_inner, ns) = store();
        let r = ns
            .put_bytes(
                &k("a.bin"),
                Bytes::from_static(b"hello"),
                PutOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(drain(ns.get(&r, None).await.unwrap()).await, b"hello");
        let m = ns.head(&r).await.unwrap();
        assert_eq!(m.size, 5);
        ns.delete(&r).await.unwrap();
        assert!(matches!(
            ns.head(&r).await.unwrap_err(),
            BlobError::NotFound
        ));
    }

    #[tokio::test]
    async fn list_only_returns_entries_inside_namespace() {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner.clone(), "tenant-7/").unwrap();
        // Outsider writes a key not under the namespace.
        inner
            .put_bytes(
                &k("tenant-8/secret"),
                Bytes::from_static(b"x"),
                PutOptions::default(),
            )
            .await
            .unwrap();
        // Insider writes via the wrapper.
        ns.put_bytes(&k("a"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        ns.put_bytes(&k("b"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        let page = ns.list(None, None).await.unwrap();
        assert_eq!(page.items.len(), 2, "must not leak tenant-8 row");
    }

    #[tokio::test]
    async fn quota_rejects_put_bytes_that_would_overflow() {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner, "tenant-7/")
            .unwrap()
            .with_quota(Quota::max_bytes(10));
        // 6 bytes — fits.
        ns.put_bytes(&k("a"), Bytes::from_static(b"abcdef"), PutOptions::default())
            .await
            .unwrap();
        // Adding 5 more would push us to 11; reject.
        let err = ns
            .put_bytes(&k("b"), Bytes::from_static(b"vwxyz"), PutOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, BlobError::PayloadTooLarge));
        // 4 still fits exactly at the cap (6 + 4 = 10).
        ns.put_bytes(&k("c"), Bytes::from_static(b"wxyz"), PutOptions::default())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn quota_rejects_on_object_count() {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner, "tenant-7/")
            .unwrap()
            .with_quota(Quota::max_objects(2));
        ns.put_bytes(&k("a"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        ns.put_bytes(&k("b"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        let err = ns
            .put_bytes(&k("c"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, BlobError::PayloadTooLarge));
    }

    #[tokio::test]
    async fn quota_isolates_namespaces() {
        // Two namespaces over the same inner store: one's writes
        // must not eat the other's cap.
        let inner = Arc::new(MemoryBlobStore::new());
        let ns7 = Namespaced::new(inner.clone(), "tenant-7/")
            .unwrap()
            .with_quota(Quota::max_bytes(4));
        let ns8 = Namespaced::new(inner, "tenant-8/")
            .unwrap()
            .with_quota(Quota::max_bytes(4));
        ns7.put_bytes(&k("a"), Bytes::from_static(b"abcd"), PutOptions::default())
            .await
            .unwrap();
        ns8.put_bytes(&k("a"), Bytes::from_static(b"wxyz"), PutOptions::default())
            .await
            .unwrap();
        assert!(matches!(
            ns7.put_bytes(&k("b"), Bytes::from_static(b"!"), PutOptions::default())
                .await
                .unwrap_err(),
            BlobError::PayloadTooLarge
        ));
    }

    #[tokio::test]
    async fn approximate_usage_forwards_combined_prefix() {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner.clone(), "tenant-7/").unwrap();
        ns.put_bytes(&k("docs/a"), Bytes::from_static(b"abcd"), PutOptions::default())
            .await
            .unwrap();
        ns.put_bytes(&k("docs/b"), Bytes::from_static(b"xy"), PutOptions::default())
            .await
            .unwrap();
        ns.put_bytes(&k("imgs/x"), Bytes::from_static(b"!"), PutOptions::default())
            .await
            .unwrap();
        // Caller asks for usage of a sub-prefix inside the
        // namespace; combinator prepends `tenant-7/` for the inner
        // store.
        let u = ns.approximate_usage(&k("docs/")).await.unwrap();
        assert_eq!(u, BlobUsage::new(6, 2));
    }

    #[tokio::test]
    async fn list_filters_by_caller_prefix() {
        let inner = Arc::new(MemoryBlobStore::new());
        let ns = Namespaced::new(inner, "tenant-7/").unwrap();
        for key in ["avatars/me", "avatars/you", "docs/x"] {
            ns.put_bytes(&k(key), Bytes::from_static(b"x"), PutOptions::default())
                .await
                .unwrap();
        }
        let page = ns.list(Some(&k("avatars/")), None).await.unwrap();
        assert_eq!(page.items.len(), 2);
    }
}
