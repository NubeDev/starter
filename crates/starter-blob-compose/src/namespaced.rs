//! `Namespaced` — prefix-isolating combinator.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore,
    ListPage, PresignOp, PresignedUrl, PutOptions,
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
        })
    }

    /// The prefix this combinator prepends. Operator-facing
    /// diagnostic only — not part of the consumer surface.
    pub fn prefix(&self) -> &str {
        &self.prefix
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

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
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
        let inner_src = self.unwrap_ref(src)?;
        let inner_dst = self.combined(dst_key)?;
        let inner_ref = self.inner.copy_server_side(&inner_src, &inner_dst).await?;
        Ok(self.wrap(inner_ref))
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
