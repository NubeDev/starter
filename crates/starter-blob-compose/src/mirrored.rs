//! `Mirrored` — write-fan-out combinator.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{BoxStream, TryStreamExt};
use serde::{Deserialize, Serialize};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore,
    ListPage, PresignOp, PresignedUrl, PutOptions,
};

/// Durability mode for [`Mirrored`].
///
/// The name of the variant is the durability contract — load-bearing
/// for B3. A consumer reading the wiring code knows exactly what
/// they bought without cross-referencing the docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorMode {
    /// Synchronous: a `put` returns only after every mirror has
    /// acknowledged. Any mirror error fails the put — the consumer
    /// sees a `Result::Err` and can retry. The trade-off is
    /// latency: the slowest mirror is the put's tail latency.
    Sync,

    /// Best-effort: a `put` returns after the **primary** succeeds.
    /// Mirror writes are spawned in the background and may silently
    /// fail (logged at `WARN`). The consumer's durability promise
    /// is exactly *primary*-level — the mirrors are a recovery
    /// nicety, not part of the durability story. The variant name
    /// spells this out so a code reviewer cannot miss it (B3).
    AsyncBackground,
}

/// Writes to a primary plus zero or more mirrors. Reads dispatch
/// to the primary only.
///
/// # Locator shape
///
/// `BlobRef::opaque_locator` wraps the **primary**'s inner ref.
/// Mirrors are write-only — a read against the `Mirrored` never
/// reaches them. Why: we want a deterministic read path
/// (latency, error surface) and we want the primary to be the
/// single source of truth on `delete`. Asymmetric mirror reads
/// would invite eventual-consistency surprises a consumer would
/// have to reason about; we refuse to ship that.
///
/// # Failure modes (B3 — durability does not shift)
///
/// - [`MirrorMode::Sync`]: any mirror put failure fails the call.
/// - [`MirrorMode::AsyncBackground`]: primary failure fails the
///   call; mirror failures are logged and dropped. The consumer
///   sees primary durability, period.
/// - `get` / `head` / `presign` / `delete` always hit the primary.
///   Mirrors are not consulted on read; a stale mirror is not a
///   correctness problem because the read path never sees it.
/// - A `delete` does not propagate to mirrors. The justification:
///   `Mirrored` exists to add redundancy, not to coordinate
///   deletes across geographies. A consumer who needs cross-mirror
///   deletes should compose `Mirrored<Tiered<...>>` so the inner
///   `Tiered` carries the delete-everywhere contract.
pub struct Mirrored {
    primary: Arc<dyn BlobStore>,
    mirrors: Vec<Arc<dyn BlobStore>>,
    mode: MirrorMode,
    backend_id: BackendId,
}

#[derive(Serialize, Deserialize)]
struct Locator {
    primary: BlobRef,
}

/// Builder for [`Mirrored`]. Use [`Mirrored::builder`] to start.
pub struct MirroredBuilder {
    primary: Arc<dyn BlobStore>,
    mirrors: Vec<Arc<dyn BlobStore>>,
    mode: MirrorMode,
}

impl MirroredBuilder {
    /// Add a mirror. Order matters for diagnostics only.
    pub fn mirror(mut self, store: Arc<dyn BlobStore>) -> Self {
        self.mirrors.push(store);
        self
    }

    /// Set the durability mode. Defaults to [`MirrorMode::Sync`].
    pub fn mode(mut self, mode: MirrorMode) -> Self {
        self.mode = mode;
        self
    }

    /// Finish the builder.
    pub fn build(self) -> Mirrored {
        let backend_id = BackendId::new(format!(
            "mirrored({},mirrors={},mode={:?})",
            self.primary.backend_id().as_str(),
            self.mirrors.len(),
            self.mode,
        ));
        Mirrored {
            primary: self.primary,
            mirrors: self.mirrors,
            mode: self.mode,
            backend_id,
        }
    }
}

impl Mirrored {
    /// Start building a `Mirrored` over the supplied primary.
    pub fn builder(primary: Arc<dyn BlobStore>) -> MirroredBuilder {
        MirroredBuilder {
            primary,
            mirrors: Vec::new(),
            mode: MirrorMode::Sync,
        }
    }

    fn wrap(&self, primary: BlobRef) -> BlobRef {
        let size = primary.size();
        let etag = primary.etag().clone();
        let loc = crate::codec::encode(Locator { primary });
        BlobRef::mint(self.backend_id.clone(), loc, etag, size)
    }

    fn unwrap_primary(&self, outer: &BlobRef) -> Result<BlobRef, BlobError> {
        let l: Locator = crate::codec::decode(outer.opaque_locator())?;
        Ok(l.primary)
    }
}

async fn fan_out_bytes(
    mirrors: &[Arc<dyn BlobStore>],
    key: &BlobKey,
    bytes: Bytes,
    opts: PutOptions,
) -> Result<(), BlobError> {
    for m in mirrors {
        m.put_bytes(key, bytes.clone(), opts.clone()).await?;
    }
    Ok(())
}

#[async_trait]
impl BlobStore for Mirrored {
    fn backend_id(&self) -> &BackendId {
        &self.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        let primary_ref = self
            .primary
            .put_bytes(key, bytes.clone(), opts.clone())
            .await?;
        match self.mode {
            MirrorMode::Sync => {
                fan_out_bytes(&self.mirrors, key, bytes, opts).await?;
            }
            MirrorMode::AsyncBackground => {
                let mirrors = self.mirrors.clone();
                let key_owned = key.clone();
                tokio::spawn(async move {
                    if let Err(e) = fan_out_bytes(&mirrors, &key_owned, bytes, opts).await {
                        tracing::warn!(
                            target: crate::TRACE_TARGET,
                            error = %e,
                            "async mirror put failed; primary write is durable"
                        );
                    }
                });
            }
        }
        Ok(self.wrap(primary_ref))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // Streams cannot be cheaply tee'd; collect once, then
        // delegate to put_bytes. The cost is documented in the
        // type-level docs — `Mirrored` over `put_stream` buffers
        // in memory. An operator who needs zero-buffer mirrored
        // streaming should run the mirror at the network layer
        // (e.g. an S3 replication rule) rather than in the SPI.
        let chunks: Vec<Bytes> = stream.try_collect().await?;
        let total: usize = chunks.iter().map(|b| b.len()).sum();
        let mut buf = Vec::with_capacity(total);
        for c in chunks {
            buf.extend_from_slice(&c);
        }
        self.put_bytes(key, Bytes::from(buf), opts).await
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let primary = self.unwrap_primary(blob_ref)?;
        self.primary.get(&primary, range).await
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let primary = self.unwrap_primary(blob_ref)?;
        self.primary.head(&primary).await
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let primary = self.unwrap_primary(blob_ref)?;
        self.primary.delete(&primary).await
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        let page = self.primary.list(prefix, cursor).await?;
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
        let primary = self.unwrap_primary(blob_ref)?;
        self.primary.presign(&primary, op, ttl).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn sync_mirror_writes_to_both() {
        let primary = mem("p");
        let mirror = mem("m");
        let m = Mirrored::builder(primary.clone())
            .mirror(mirror.clone())
            .mode(MirrorMode::Sync)
            .build();
        let _ = m
            .put_bytes(&k("x"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        assert_eq!(primary.list(None, None).await.unwrap().items.len(), 1);
        assert_eq!(mirror.list(None, None).await.unwrap().items.len(), 1);
    }

    /// A mirror store that always returns Timeout, so we can prove
    /// the durability promise of each mode.
    struct FailingStore {
        id: BackendId,
    }

    impl FailingStore {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                id: BackendId::new("failing"),
            })
        }
    }

    #[async_trait]
    impl BlobStore for FailingStore {
        fn backend_id(&self) -> &BackendId {
            &self.id
        }
        async fn put_bytes(
            &self,
            _: &BlobKey,
            _: Bytes,
            _: PutOptions,
        ) -> Result<BlobRef, BlobError> {
            Err(BlobError::Timeout)
        }
        async fn put_stream(
            &self,
            _: &BlobKey,
            _: BoxStream<'static, Result<Bytes, BlobError>>,
            _: PutOptions,
        ) -> Result<BlobRef, BlobError> {
            Err(BlobError::Timeout)
        }
        async fn get(
            &self,
            _: &BlobRef,
            _: Option<BlobRange>,
        ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
            Err(BlobError::NotFound)
        }
        async fn head(&self, _: &BlobRef) -> Result<BlobMeta, BlobError> {
            Err(BlobError::NotFound)
        }
        async fn delete(&self, _: &BlobRef) -> Result<(), BlobError> {
            Ok(())
        }
        async fn list(&self, _: Option<&BlobKey>, _: Option<&str>) -> Result<ListPage, BlobError> {
            Ok(ListPage::default())
        }
        async fn presign(
            &self,
            _: &BlobRef,
            _: PresignOp,
            _: Duration,
        ) -> Result<PresignedUrl, BlobError> {
            Err(BlobError::Unsupported)
        }
    }

    #[tokio::test]
    async fn sync_mirror_failure_fails_the_put() {
        let primary = mem("p");
        let m = Mirrored::builder(primary.clone())
            .mirror(FailingStore::new())
            .mode(MirrorMode::Sync)
            .build();
        let err = m
            .put_bytes(&k("x"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, BlobError::Timeout));
    }

    #[tokio::test]
    async fn async_mirror_failure_does_not_fail_the_put() {
        let primary = mem("p");
        let m = Mirrored::builder(primary.clone())
            .mirror(FailingStore::new())
            .mode(MirrorMode::AsyncBackground)
            .build();
        // Primary succeeds; mirror would fail in the background.
        let r = m
            .put_bytes(&k("x"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        assert_eq!(m.head(&r).await.unwrap().size, 2);
    }
}
