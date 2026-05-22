//! `ReadThroughCache` — lazy read-side cache.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore,
    ListPage, PresignOp, PresignedUrl, PutOptions,
};

/// Read-through cache: writes go to the source only; reads try
/// the cache and populate it from the source on miss.
///
/// # Why this is not a [`Tiered`]
///
/// `Tiered` decides where the *write* lands per policy; this
/// combinator never writes to the cache as part of `put` — only as
/// a side-effect of a `get`. Folding the two would force a
/// `WriteTarget` enum on the combined type, and no consumer wants
/// that today. Keeping them separate keeps each type's contract
/// readable in one breath.
///
/// # Locator shape
///
/// `BlobRef::opaque_locator` wraps the **source** inner ref. Cache
/// content is keyed by the same BlobKey through the source's
/// `BlobKey` (the consumer-supplied key the source saw), but the
/// returned outer ref always routes future reads through the same
/// cache+source pair.
///
/// # Failure modes (B3 — durability does not shift)
///
/// - `put` writes to the source only. Cache is not populated on
///   write; doing so would lock the consumer into a write-through
///   model and `ReadThroughCache` would no longer describe the
///   shape on the tin.
/// - `get` consults the cache first. On hit, the cache's bytes are
///   returned; on miss, the source is read AND a cache write is
///   attempted (best-effort; a cache failure does not fail the
///   read).
/// - **After `delete`, the cache is also cleared.** Without this,
///   a stale cache entry would survive the source delete and a
///   subsequent `get` would return bytes the consumer asked us to
///   remove — a clear B3 violation ("`ReadThroughCache` is never
///   the source of truth on read after delete," per the SCOPE).
/// - `head` / `list` / `presign` go to the source. The cache may
///   hold a different etag than the source after a write churns
///   it; consumers must trust the source for canonical metadata.
pub struct ReadThroughCache {
    source: Arc<dyn BlobStore>,
    cache: Arc<dyn BlobStore>,
    ttl: Option<Duration>,
    backend_id: BackendId,
}

#[derive(Serialize, Deserialize)]
struct Locator {
    /// Source-side ref.
    source: BlobRef,
    /// The BlobKey the source minted under. Stored so we can re-
    /// populate (or invalidate) the cache without reconstructing a
    /// key from the source's opaque locator. This is *not* a B2
    /// violation: the key sits inside an opaque locator with no
    /// public accessor — the consumer cannot read it.
    cache_key: String,
}

impl ReadThroughCache {
    /// Build a `ReadThroughCache` over `source` populated lazily
    /// into `cache`. `ttl` is advisory and only honoured by cache
    /// backends that support expiry (the memory engine ignores
    /// it).
    pub fn new(
        source: Arc<dyn BlobStore>,
        cache: Arc<dyn BlobStore>,
        ttl: Option<Duration>,
    ) -> Self {
        let backend_id = BackendId::new(format!(
            "read-through-cache(source={},cache={})",
            source.backend_id().as_str(),
            cache.backend_id().as_str(),
        ));
        Self {
            source,
            cache,
            ttl,
            backend_id,
        }
    }

    fn wrap(&self, source: BlobRef, cache_key: String) -> BlobRef {
        let size = source.size();
        let etag = source.etag().clone();
        let loc = crate::codec::encode(Locator { source, cache_key });
        BlobRef::mint(self.backend_id.clone(), loc, etag, size)
    }

    fn unpack(&self, outer: &BlobRef) -> Result<Locator, BlobError> {
        crate::codec::decode(outer.opaque_locator())
    }

    /// Returns the advisory cache TTL set at construction time.
    /// Exposed for operator-facing diagnostics; not part of the
    /// trait surface.
    pub fn ttl(&self) -> Option<Duration> {
        self.ttl
    }
}

#[async_trait]
impl BlobStore for ReadThroughCache {
    fn backend_id(&self) -> &BackendId {
        &self.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // Source-only write. Invalidate any stale cache entry under
        // this key so the next read repopulates from source rather
        // than serving stale bytes.
        let source_ref = self.source.put_bytes(key, bytes, opts).await?;
        invalidate_cache(&self.cache, key).await;
        Ok(self.wrap(source_ref, key.as_str().to_owned()))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        let source_ref = self.source.put_stream(key, stream, opts).await?;
        invalidate_cache(&self.cache, key).await;
        Ok(self.wrap(source_ref, key.as_str().to_owned()))
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let Locator { source, cache_key } = self.unpack(blob_ref)?;
        let key = BlobKey::new(cache_key.clone()).map_err(BlobError::backend)?;

        // Try the cache by listing-with-prefix. We avoid plumbing
        // a side-channel cache-ref through the locator because the
        // cache engine may have evicted between writes — the cache
        // is the *source of truth nowhere*, and we look it up by
        // BlobKey rather than by a stored ref to keep that
        // invariant honest.
        if let Some(cache_ref) = lookup_cache_ref(&self.cache, &key).await? {
            if let Ok(s) = self.cache.get(&cache_ref, range).await {
                return Ok(s);
            }
        }

        // Cache miss: read source, populate cache lazily.
        let stream = self.source.get(&source, None).await?;
        let chunks: Vec<Bytes> = stream.try_collect().await?;
        let total: usize = chunks.iter().map(|b| b.len()).sum();
        let mut buf = Vec::with_capacity(total);
        for c in chunks {
            buf.extend_from_slice(&c);
        }
        let payload = Bytes::from(buf);

        // Best-effort cache populate; a failure does not fail the
        // read. The TTL knob is advisory; the memory cache ignores
        // it. Engines that honour expiry should consume `self.ttl`
        // via PutOptions extensions when they grow them.
        if let Err(e) = self
            .cache
            .put_bytes(&key, payload.clone(), PutOptions::default())
            .await
        {
            tracing::warn!(
                target: crate::TRACE_TARGET,
                error = %e,
                "cache populate failed; read served from source"
            );
        }

        // Honour the range request on the buffer we just materialised.
        let out = match range {
            Some(r) => {
                let len = payload.len() as u64;
                if r.start >= len {
                    Bytes::new()
                } else {
                    let end = r.end.min(len - 1) as usize;
                    let start = r.start as usize;
                    payload.slice(start..=end)
                }
            }
            None => payload,
        };
        Ok(stream::once(async move { Ok(out) }).boxed())
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let l = self.unpack(blob_ref)?;
        self.source.head(&l.source).await
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let l = self.unpack(blob_ref)?;
        let key = BlobKey::new(l.cache_key.clone()).map_err(BlobError::backend)?;
        // Source delete is the authoritative step. We clear the
        // cache too — per B3, a stale cache entry must never
        // out-live a source delete.
        self.source.delete(&l.source).await?;
        invalidate_cache(&self.cache, &key).await;
        Ok(())
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        let page = self.source.list(prefix, cursor).await?;
        let items = page
            .items
            .into_iter()
            .map(|(r, m)| {
                let cache_key = r.opaque_locator().to_owned();
                (self.wrap(r, cache_key), m)
            })
            .collect();
        Ok(ListPage::new(items, page.next_cursor))
    }

    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        let l = self.unpack(blob_ref)?;
        self.source.presign(&l.source, op, ttl).await
    }
}

async fn lookup_cache_ref(
    cache: &Arc<dyn BlobStore>,
    key: &BlobKey,
) -> Result<Option<BlobRef>, BlobError> {
    // Use the prefix-list to find an exact-key match. This works
    // across every starter engine because list() guarantees
    // prefix semantics. It is O(1) for the memory engine and O(1)
    // amortised for the fs engine on a single-element prefix.
    let page = cache.list(Some(key), None).await?;
    Ok(page
        .items
        .into_iter()
        .find(|(r, _)| r.opaque_locator() == key.as_str())
        .map(|(r, _)| r))
}

async fn invalidate_cache(cache: &Arc<dyn BlobStore>, key: &BlobKey) {
    if let Ok(Some(r)) = lookup_cache_ref(cache, key).await {
        if let Err(e) = cache.delete(&r).await {
            tracing::warn!(
                target: crate::TRACE_TARGET,
                error = %e,
                "cache invalidate failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use starter_blob_memory::{MemoryBlobStore, MemoryBlobStoreConfig};

    fn mem(id: &str) -> Arc<MemoryBlobStore> {
        Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
            backend_id: BackendId::new(id),
            ..Default::default()
        }))
    }

    fn k(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    async fn drain(s: BoxStream<'static, Result<Bytes, BlobError>>) -> Vec<u8> {
        let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
        chunks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    #[tokio::test]
    async fn read_populates_cache_lazily() {
        let source = mem("src");
        let cache = mem("cache");
        let rtc = ReadThroughCache::new(source.clone(), cache.clone(), None);
        let r = rtc
            .put_bytes(&k("x"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        // Write went to source only.
        assert_eq!(cache.list(None, None).await.unwrap().items.len(), 0);
        // First read populates cache.
        let _ = drain(rtc.get(&r, None).await.unwrap()).await;
        assert_eq!(cache.list(None, None).await.unwrap().items.len(), 1);
    }

    #[tokio::test]
    async fn delete_clears_cache_too() {
        let source = mem("src");
        let cache = mem("cache");
        let rtc = ReadThroughCache::new(source.clone(), cache.clone(), None);
        let r = rtc
            .put_bytes(&k("x"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        // Pull once to populate cache.
        let _ = drain(rtc.get(&r, None).await.unwrap()).await;
        assert_eq!(cache.list(None, None).await.unwrap().items.len(), 1);
        rtc.delete(&r).await.unwrap();
        assert_eq!(
            cache.list(None, None).await.unwrap().items.len(),
            0,
            "B3: stale cache entry must not survive source delete"
        );
    }
}
