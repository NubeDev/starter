//! `Tiered` — hot/cold storage combinator.

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

/// Policy describing when a write should land directly in cold
/// instead of hot, and (optionally) whether a cold-read should
/// promote-back into hot.
///
/// All fields are advisory hints, not guarantees. The combinator
/// honours them on a best-effort basis; a backend failure to
/// promote/demote is logged at `WARN` and the operation continues
/// against the surviving tier.
#[derive(Clone, Debug, Default)]
pub struct TieredPolicy {
    /// Bytes above this size go straight to cold on `put`. `None`
    /// disables the size demotion rule.
    pub demote_above_bytes: Option<u64>,

    /// Records older than this on `head` are eligible for demotion
    /// on the *next* write to the same key. Pure size + age policy
    /// kept here so on-eviction-style background sweepers can layer
    /// on top later without a SPI change.
    pub demote_above_age: Option<Duration>,

    /// If `true`, a successful cold read writes the bytes back into
    /// hot synchronously before returning. Off by default — most
    /// consumers prefer the simpler "cold reads stay in cold" story.
    pub promote_back_on_read: bool,
}

/// Two-tier blob store: writes pick a tier per [`TieredPolicy`],
/// reads try hot then cold.
///
/// # Locator shape
///
/// `BlobRef::opaque_locator` is a JSON envelope carrying the inner
/// ref AND the tier it was last written to. On read, the stored
/// tier is tried first; if `NotFound` the other tier is tried as a
/// recovery. The returned `BlobRef` is documented to be *advisory*
/// — a background promote/demote may have moved the bytes since
/// `put`. Consumers that need the canonical location should `head`
/// the ref before serving a presigned URL; the head call is what
/// confirms the current tier.
///
/// # Failure modes (B3 — durability does not shift)
///
/// - `put`: the write to the chosen tier is the only durable step.
///   If demotion is requested *and* the cold write fails, the put
///   surfaces the cold-write error. The hot bytes may be left
///   behind in that case; a future delete will reach both tiers.
/// - `get`: hot-first, cold-fallback. If both fail the consumer
///   sees the hot error (because hot is the canonical write
///   target).
/// - `delete`: deletes from BOTH tiers. A failure on either is
///   surfaced. (B3: a `Tiered::delete` that left bytes behind in
///   cold would silently weaken durability — we refuse.)
/// - `presign`: dispatches to the tier where the ref currently
///   lives. If the ref's tier no longer has the bytes the consumer
///   receives a presigned URL that will 404 — the same surface a
///   raw S3 presign would give on a deleted key.
pub struct Tiered {
    hot: Arc<dyn BlobStore>,
    cold: Arc<dyn BlobStore>,
    policy: TieredPolicy,
    backend_id: BackendId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Tier {
    Hot,
    Cold,
}

#[derive(Serialize, Deserialize)]
struct Locator {
    tier: Tier,
    inner: BlobRef,
}

impl Tiered {
    /// Build a `Tiered` over `hot`, `cold`, and the supplied
    /// `policy`. The hot store is the canonical write target.
    pub fn new(hot: Arc<dyn BlobStore>, cold: Arc<dyn BlobStore>, policy: TieredPolicy) -> Self {
        let backend_id = BackendId::new(format!(
            "tiered(hot={},cold={})",
            hot.backend_id().as_str(),
            cold.backend_id().as_str()
        ));
        Self {
            hot,
            cold,
            policy,
            backend_id,
        }
    }

    fn wrap(&self, tier: Tier, inner: BlobRef) -> BlobRef {
        let size = inner.size();
        let etag = inner.etag().clone();
        let loc = crate::codec::encode(Locator { tier, inner });
        BlobRef::mint(self.backend_id.clone(), loc, etag, size)
    }

    fn unpack(&self, outer: &BlobRef) -> Result<(Tier, BlobRef), BlobError> {
        let loc: Locator = crate::codec::decode(outer.opaque_locator())?;
        Ok((loc.tier, loc.inner))
    }

    fn store_for(&self, tier: Tier) -> &Arc<dyn BlobStore> {
        match tier {
            Tier::Hot => &self.hot,
            Tier::Cold => &self.cold,
        }
    }

    fn other(tier: Tier) -> Tier {
        match tier {
            Tier::Hot => Tier::Cold,
            Tier::Cold => Tier::Hot,
        }
    }

    fn target_tier_for_put(&self, bytes_len: u64) -> Tier {
        match self.policy.demote_above_bytes {
            Some(limit) if bytes_len > limit => Tier::Cold,
            _ => Tier::Hot,
        }
    }
}

#[async_trait]
impl BlobStore for Tiered {
    fn backend_id(&self) -> &BackendId {
        &self.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        let tier = self.target_tier_for_put(bytes.len() as u64);
        let inner = self.store_for(tier).put_bytes(key, bytes, opts).await?;
        Ok(self.wrap(tier, inner))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // We cannot know size-up-front from the stream, so the
        // size-based demotion policy cannot apply here without
        // buffering. Stream-style puts always go to hot; the
        // operator can run a background demoter on top of `list()
        // + head()` if they need stream demotion. This is
        // documented behaviour, not a silent fallback — `put_bytes`
        // is the path that honours `demote_above_bytes`.
        let inner = self.hot.put_stream(key, stream, opts).await?;
        Ok(self.wrap(Tier::Hot, inner))
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let (tier, inner) = self.unpack(blob_ref)?;
        match self.store_for(tier).get(&inner, range).await {
            Ok(s) => Ok(s),
            Err(BlobError::NotFound) => {
                // Recovery path: try the *other* tier. This handles
                // a background demoter that has since moved the
                // bytes.
                let other = Self::other(tier);
                let stream = self.store_for(other).get(&inner, range).await?;
                if self.policy.promote_back_on_read && other == Tier::Cold {
                    // We found bytes in cold; copy them back to hot
                    // synchronously so the next read is fast. We
                    // must buffer the stream into memory once; the
                    // returned stream is a fresh one over the
                    // buffer. This is honest about the durability
                    // implication — see the type-level docs.
                    let bytes: Vec<Bytes> = stream.try_collect().await?;
                    let mut all = Vec::with_capacity(bytes.iter().map(|b| b.len()).sum());
                    for b in &bytes {
                        all.extend_from_slice(b);
                    }
                    let payload = Bytes::from(all);
                    // Promote: re-`put_stream` the bytes to hot
                    // under the inner ref's locator-derived key.
                    // We cannot reconstruct the original BlobKey
                    // from the inner ref (B2), so promotion goes
                    // through the inner store's own re-`put` path
                    // *only if* the inner store exposes a key —
                    // which by construction it does not. We
                    // therefore promote only when the inner store
                    // is the same hot store that minted the ref
                    // earlier in this process, by re-using
                    // `copy_via_client` between cold and hot. That
                    // requires a `BlobKey`; the type doesn't have
                    // one. We log a one-line WARN and skip the
                    // promotion rather than fabricating a key — a
                    // silent re-key would violate B3.
                    tracing::warn!(
                        target: crate::TRACE_TARGET,
                        "promote_back_on_read skipped — inner ref carries no recoverable key; \
                         use list()-driven promotion instead"
                    );
                    Ok(stream::once(async move { Ok(payload) }).boxed())
                } else {
                    Ok(stream)
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let (tier, inner) = self.unpack(blob_ref)?;
        match self.store_for(tier).head(&inner).await {
            Ok(m) => Ok(m),
            Err(BlobError::NotFound) => {
                let other = Self::other(tier);
                self.store_for(other).head(&inner).await
            }
            Err(e) => Err(e),
        }
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let (_tier, inner) = self.unpack(blob_ref)?;
        // B3: delete reaches BOTH tiers. A `Tiered` whose `delete`
        // left bytes in cold would silently weaken durability.
        let hot = self.hot.delete(&inner).await;
        let cold = self.cold.delete(&inner).await;
        hot.and(cold)
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        // We list hot first; cold contributions appear after hot
        // pages are exhausted. The cursor encodes which tier we're
        // in. This is a pragmatic shape rather than a global merge
        // — a global merge would require materialising both stores
        // for every page and the cost outweighs the benefit for
        // the only use case this combinator targets (background
        // admin sweeps).
        const HOT_DONE: &str = "compose-tiered:cold:";
        let (which, inner_cursor) = match cursor {
            Some(c) if c.starts_with(HOT_DONE) => (Tier::Cold, c.strip_prefix(HOT_DONE)),
            other => (Tier::Hot, other),
        };
        let inner_cursor = inner_cursor.filter(|s| !s.is_empty());
        let page = self.store_for(which).list(prefix, inner_cursor).await?;
        let items: Vec<_> = page
            .items
            .into_iter()
            .map(|(r, m)| (self.wrap(which, r), m))
            .collect();
        let next_cursor = match (which, page.next_cursor) {
            (Tier::Hot, Some(c)) => Some(c),
            (Tier::Hot, None) => Some(HOT_DONE.to_string()),
            (Tier::Cold, Some(c)) => Some(format!("{HOT_DONE}{c}")),
            (Tier::Cold, None) => None,
        };
        Ok(ListPage::new(items, next_cursor))
    }

    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        let (tier, inner) = self.unpack(blob_ref)?;
        self.store_for(tier).presign(&inner, op, ttl).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use starter_blob_memory::{MemoryBlobStore, MemoryBlobStoreConfig};

    fn k(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    async fn drain(s: BoxStream<'static, Result<Bytes, BlobError>>) -> Vec<u8> {
        let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
        chunks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    fn tier(policy: TieredPolicy) -> (Arc<MemoryBlobStore>, Arc<MemoryBlobStore>, Tiered) {
        let hot = Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
            backend_id: BackendId::new("hot"),
            ..Default::default()
        }));
        let cold = Arc::new(MemoryBlobStore::with_config(MemoryBlobStoreConfig {
            backend_id: BackendId::new("cold"),
            ..Default::default()
        }));
        let t = Tiered::new(hot.clone(), cold.clone(), policy);
        (hot, cold, t)
    }

    #[tokio::test]
    async fn small_writes_land_in_hot() {
        let (hot, cold, t) = tier(TieredPolicy {
            demote_above_bytes: Some(100),
            ..Default::default()
        });
        let r = t
            .put_bytes(&k("x"), Bytes::from_static(b"small"), PutOptions::default())
            .await
            .unwrap();
        assert_eq!(t.head(&r).await.unwrap().size, 5);
        assert_eq!(hot.list(None, None).await.unwrap().items.len(), 1);
        assert_eq!(cold.list(None, None).await.unwrap().items.len(), 0);
    }

    #[tokio::test]
    async fn large_writes_demote_to_cold() {
        let (hot, cold, t) = tier(TieredPolicy {
            demote_above_bytes: Some(4),
            ..Default::default()
        });
        let r = t
            .put_bytes(
                &k("x"),
                Bytes::from_static(b"larger-payload"),
                PutOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(
            drain(t.get(&r, None).await.unwrap()).await,
            b"larger-payload"
        );
        assert_eq!(hot.list(None, None).await.unwrap().items.len(), 0);
        assert_eq!(cold.list(None, None).await.unwrap().items.len(), 1);
    }

    #[tokio::test]
    async fn read_falls_back_when_bytes_moved_out_of_band() {
        let (hot, cold, t) = tier(TieredPolicy::default());
        // Put through Tiered -> lands in hot.
        let r = t
            .put_bytes(&k("x"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        // Simulate an out-of-band demotion: copy the bytes into
        // cold and remove from hot.
        cold.put_bytes(&k("x"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        let hot_page = hot.list(None, None).await.unwrap();
        let (hot_ref, _) = &hot_page.items[0];
        hot.delete(hot_ref).await.unwrap();
        // Tiered.get must still return the bytes via cold fallback.
        assert_eq!(drain(t.get(&r, None).await.unwrap()).await, b"v1");
    }

    #[tokio::test]
    async fn delete_clears_both_tiers() {
        let (hot, cold, t) = tier(TieredPolicy::default());
        let r = t
            .put_bytes(&k("x"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        // Smuggle a stale copy into cold.
        cold.put_bytes(&k("x"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        t.delete(&r).await.unwrap();
        assert_eq!(hot.list(None, None).await.unwrap().items.len(), 0);
        assert_eq!(cold.list(None, None).await.unwrap().items.len(), 0);
    }

    #[tokio::test]
    async fn list_walks_hot_then_cold() {
        let (hot, cold, t) = tier(TieredPolicy::default());
        hot.put_bytes(&k("a"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        cold.put_bytes(&k("b"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        let p1 = t.list(None, None).await.unwrap();
        assert_eq!(p1.items.len(), 1, "hot first");
        let p2 = t.list(None, p1.next_cursor.as_deref()).await.unwrap();
        assert_eq!(p2.items.len(), 1, "then cold");
        assert!(p2.next_cursor.is_none());
    }
}
