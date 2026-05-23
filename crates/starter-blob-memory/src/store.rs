//! `MemoryBlobStore` — `HashMap<String, Entry>` behind an `RwLock`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};
use rand::RngCore;
use sha2::{Digest, Sha256};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore, Etag,
    ListPage, PresignOp, PresignedUrl, PutOptions,
};
use tokio::sync::RwLock;
use tracing::debug;

use crate::presign::{self, PresignClaim};
use crate::TRACE_TARGET;

/// Build-time configuration for [`MemoryBlobStore`].
///
/// `backend_id` is stamped onto every minted [`BlobRef`]. `public_base_url`
/// is the prefix the engine puts in front of presigned URLs — the URL
/// the consumer eventually serves the engine's [`crate::router`] under.
/// Defaults to `"memory://"`, which is fine for tests that go straight
/// to the router via `tower::ServiceExt::oneshot` and never hit a real
/// HTTP stack.
#[derive(Clone, Debug)]
pub struct MemoryBlobStoreConfig {
    /// Stable id reported by [`BlobStore::backend_id`]. Default
    /// `"memory:default"`.
    pub backend_id: BackendId,
    /// Base URL prepended to presigned tokens. Default `"memory://"`.
    pub public_base_url: String,
}

impl Default for MemoryBlobStoreConfig {
    fn default() -> Self {
        Self {
            backend_id: BackendId::new("memory:default"),
            public_base_url: "memory://".into(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Entry {
    pub bytes: Bytes,
    pub meta: BlobMeta,
}

pub(crate) struct Inner {
    pub backend_id: BackendId,
    pub public_base_url: String,
    pub hmac_key: [u8; 32],
    pub data: RwLock<HashMap<String, Entry>>,
}

/// In-process blob store. Cheap to clone — wraps an `Arc`.
#[derive(Clone)]
pub struct MemoryBlobStore {
    pub(crate) inner: Arc<Inner>,
}

impl MemoryBlobStore {
    /// Build a store with the default config (`memory:default`,
    /// `memory://`). The HMAC key is freshly randomised on every
    /// call: a process restart, or a fresh `MemoryBlobStore::new()`
    /// in the same process, invalidates all previously-handed-out
    /// presigned URLs. See the crate-level docs.
    pub fn new() -> Self {
        Self::with_config(MemoryBlobStoreConfig::default())
    }

    /// Build a store with a caller-supplied config. Same HMAC-key
    /// rotation semantics as [`MemoryBlobStore::new`].
    pub fn with_config(cfg: MemoryBlobStoreConfig) -> Self {
        let mut hmac_key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut hmac_key);
        Self {
            inner: Arc::new(Inner {
                backend_id: cfg.backend_id,
                public_base_url: cfg.public_base_url,
                hmac_key,
                data: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Crate-internal HMAC accessor for the router. Kept off the
    /// public surface so consumers cannot lift the key out and
    /// forge tokens.
    pub(crate) fn hmac_key(&self) -> &[u8; 32] {
        &self.inner.hmac_key
    }

    /// Crate-internal data accessor for the router. Compiled in
    /// unconditionally so the symbol exists across feature
    /// combinations; the router (feature `axum`) is the only call
    /// site.
    #[allow(dead_code)]
    pub(crate) fn data(&self) -> &RwLock<HashMap<String, Entry>> {
        &self.inner.data
    }
}

impl Default for MemoryBlobStore {
    fn default() -> Self {
        Self::new()
    }
}

fn etag_for(bytes: &[u8]) -> Etag {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    Etag::new(hex)
}

fn meta_from(bytes: &Bytes, opts: &PutOptions, prior: Option<&BlobMeta>) -> BlobMeta {
    let now = Some(Utc::now());
    BlobMeta::new(bytes.len() as u64, etag_for(bytes))
        .with_content_type(opts.content_type.clone())
        .with_cache_control(opts.cache_control.clone())
        .with_created_at(prior.and_then(|m| m.created_at).or(now))
        .with_updated_at(now)
        .with_user_metadata(opts.user_metadata.clone())
}

fn slice_range(bytes: &Bytes, range: BlobRange) -> Bytes {
    let len = bytes.len() as u64;
    if range.start >= len {
        return Bytes::new();
    }
    let end = range.end.min(len.saturating_sub(1));
    let start = range.start as usize;
    let end_inclusive = end as usize;
    bytes.slice(start..=end_inclusive)
}

#[async_trait]
impl BlobStore for MemoryBlobStore {
    fn backend_id(&self) -> &BackendId {
        &self.inner.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        debug!(target: TRACE_TARGET, key = %key, bytes = bytes.len(), "put_bytes");
        let mut data = self.inner.data.write().await;
        if opts.if_absent && data.contains_key(key.as_str()) {
            return Err(BlobError::AlreadyExists);
        }
        let prior = data.get(key.as_str()).map(|e| e.meta.clone());
        let meta = meta_from(&bytes, &opts, prior.as_ref());
        let entry = Entry {
            bytes: bytes.clone(),
            meta: meta.clone(),
        };
        data.insert(key.as_str().to_owned(), entry);
        Ok(BlobRef::mint(
            self.inner.backend_id.clone(),
            key.as_str().to_owned(),
            meta.etag,
            meta.size,
        ))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // Memory engine has nowhere to stream *to* — collect.
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
        let data = self.inner.data.read().await;
        let entry = data
            .get(blob_ref.opaque_locator())
            .ok_or(BlobError::NotFound)?;
        let bytes = match range {
            Some(r) => slice_range(&entry.bytes, r),
            None => entry.bytes.clone(),
        };
        Ok(stream::once(async move { Ok(bytes) }).boxed())
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let data = self.inner.data.read().await;
        data.get(blob_ref.opaque_locator())
            .map(|e| e.meta.clone())
            .ok_or(BlobError::NotFound)
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let mut data = self.inner.data.write().await;
        data.remove(blob_ref.opaque_locator());
        Ok(())
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        let data = self.inner.data.read().await;
        let prefix_str = prefix.map(|p| p.as_str()).unwrap_or("");
        let mut keys: Vec<&String> = data.keys().filter(|k| k.starts_with(prefix_str)).collect();
        keys.sort();
        let start = match cursor {
            Some(c) => keys
                .iter()
                .position(|k| k.as_str() > c)
                .unwrap_or(keys.len()),
            None => 0,
        };
        const PAGE: usize = 1000;
        let end = (start + PAGE).min(keys.len());
        let items = keys[start..end]
            .iter()
            .map(|k| {
                let entry = &data[*k];
                let r = BlobRef::mint(
                    self.inner.backend_id.clone(),
                    (*k).clone(),
                    entry.meta.etag.clone(),
                    entry.meta.size,
                );
                (r, entry.meta.clone())
            })
            .collect::<Vec<_>>();
        let next_cursor = (end < keys.len()).then(|| keys[end - 1].clone());
        Ok(ListPage::new(items, next_cursor))
    }

    async fn presign(
        &self,
        blob_ref: &BlobRef,
        op: PresignOp,
        ttl: Duration,
    ) -> Result<PresignedUrl, BlobError> {
        let expires_at = SystemTime::now()
            .checked_add(ttl)
            .ok_or(BlobError::Unsupported)?;
        let expires_at_unix = expires_at
            .duration_since(UNIX_EPOCH)
            .map_err(|_| BlobError::Unsupported)?
            .as_secs();
        let claim = PresignClaim {
            op,
            locator: blob_ref.opaque_locator().to_owned(),
            expires_at_unix,
        };
        let token = presign::sign(self.hmac_key(), &claim);
        let url = format!(
            "{base}{sep}token={token}",
            base = self.inner.public_base_url,
            sep = if self.inner.public_base_url.contains('?') {
                "&"
            } else {
                "?"
            },
        );
        Ok(PresignedUrl {
            url,
            method: op,
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;

    fn key(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    async fn drain(s: BoxStream<'static, Result<Bytes, BlobError>>) -> Vec<u8> {
        let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
        chunks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    #[tokio::test]
    async fn put_get_head_delete_roundtrip() {
        let store = MemoryBlobStore::new();
        let r = store
            .put_bytes(
                &key("a/b.txt"),
                Bytes::from_static(b"hello"),
                PutOptions::with_content_type("text/plain"),
            )
            .await
            .unwrap();
        assert_eq!(r.size(), 5);
        let meta = store.head(&r).await.unwrap();
        assert_eq!(meta.content_type.as_deref(), Some("text/plain"));
        let got = drain(store.get(&r, None).await.unwrap()).await;
        assert_eq!(got, b"hello");
        store.delete(&r).await.unwrap();
        assert!(matches!(
            store.head(&r).await.unwrap_err(),
            BlobError::NotFound
        ));
    }

    #[tokio::test]
    async fn if_absent_rejects_overwrite() {
        let store = MemoryBlobStore::new();
        let k = key("x");
        store
            .put_bytes(&k, Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        let err = store
            .put_bytes(
                &k,
                Bytes::from_static(b"v2"),
                PutOptions::default().if_absent(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, BlobError::AlreadyExists));
    }

    #[tokio::test]
    async fn range_get() {
        let store = MemoryBlobStore::new();
        let r = store
            .put_bytes(
                &key("k"),
                Bytes::from_static(b"abcdefghij"),
                PutOptions::default(),
            )
            .await
            .unwrap();
        let part = drain(
            store
                .get(&r, Some(BlobRange::new(2, 4).unwrap()))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(part, b"cde");
    }

    #[tokio::test]
    async fn list_paginates_lexicographic() {
        let store = MemoryBlobStore::new();
        for c in ["c/3", "a/1", "b/2"] {
            store
                .put_bytes(&key(c), Bytes::from_static(b"x"), PutOptions::default())
                .await
                .unwrap();
        }
        let page = store.list(None, None).await.unwrap();
        let keys: Vec<_> = page
            .items
            .iter()
            .map(|(r, _)| r.opaque_locator().to_owned())
            .collect();
        assert_eq!(keys, vec!["a/1", "b/2", "c/3"]);
    }

    #[tokio::test]
    async fn list_filters_by_prefix_via_blobref_not_string() {
        let store = MemoryBlobStore::new();
        for c in ["tenants/7/a", "tenants/7/b", "tenants/8/a"] {
            store
                .put_bytes(&key(c), Bytes::from_static(b"x"), PutOptions::default())
                .await
                .unwrap();
        }
        let page = store.list(Some(&key("tenants/7/")), None).await.unwrap();
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn presign_token_verifies_against_same_store() {
        let store = MemoryBlobStore::new();
        let r = store
            .put_bytes(&key("k"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        let url = store
            .presign(&r, PresignOp::Get, Duration::from_secs(30))
            .await
            .unwrap();
        let token = url.url.split("token=").nth(1).expect("token in url");
        let claim = presign::verify(store.hmac_key(), token).unwrap();
        assert_eq!(claim.locator, "k");
        assert_eq!(claim.op, PresignOp::Get);
    }

    #[tokio::test]
    async fn presign_token_is_rejected_by_a_different_store() {
        let s1 = MemoryBlobStore::new();
        let s2 = MemoryBlobStore::new();
        let r = s1
            .put_bytes(&key("k"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        let url = s1
            .presign(&r, PresignOp::Get, Duration::from_secs(30))
            .await
            .unwrap();
        let token = url.url.split("token=").nth(1).unwrap();
        // Process-local rotation: a sibling store has its own key.
        assert!(presign::verify(s2.hmac_key(), token).is_err());
    }
}
