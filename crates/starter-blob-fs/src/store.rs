//! `FsBlobStore` — filesystem-backed `BlobStore` implementation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use chrono::{DateTime, Utc};
use futures::stream::{self, BoxStream, StreamExt, TryStreamExt};
use sha2::{Digest, Sha256};
use starter_spi::blob::{
    BackendId, BlobError, BlobKey, BlobMeta, BlobRange, BlobRef, BlobRefInternal, BlobStore,
    BlobUsage, Etag, ListPage, PresignOp, PresignedUrl, PutOptions,
};
use tempfile::NamedTempFile;
use tokio::sync::Mutex;
use tracing::debug;
use walkdir::WalkDir;

use crate::presign::{self, PresignClaim, PresignKey};
use crate::TRACE_TARGET;

/// Sidecar metadata written next to each blob. Tracks the content
/// type and timestamps the filesystem cannot honestly report on
/// every platform (notably `created_at` on Linux ext4).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Sidecar {
    content_type: Option<String>,
    cache_control: Option<String>,
    etag: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Errors specific to constructing or operating an [`FsBlobStore`].
/// Once built, the trait surface returns [`BlobError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FsBlobStoreError {
    /// I/O error while reaching the disk during setup.
    #[error("fs blob store I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// `root` exists but is not a directory.
    #[error("fs blob store root {0:?} is not a directory")]
    NotADirectory(PathBuf),
}

/// Build-time configuration. `backend_id` is stamped onto every
/// minted [`BlobRef`]; `public_base_url` is prefixed to presigned
/// URLs (defaults to `"file://"`, which is fine for tests that hit
/// the router via `oneshot`); `max_depth` bounds [`BlobStore::list`]
/// recursion.
#[derive(Clone, Debug)]
pub struct FsBlobStoreConfig {
    /// Stable id reported by [`BlobStore::backend_id`].
    pub backend_id: BackendId,
    /// Prefix prepended to presigned URLs.
    pub public_base_url: String,
    /// Walkdir cap. `None` means "no limit"; defaults to a
    /// generous 32 because deep directory trees usually mean a
    /// caller has accidentally encoded a tree into the key space.
    pub max_depth: Option<usize>,
}

impl Default for FsBlobStoreConfig {
    fn default() -> Self {
        Self {
            backend_id: BackendId::new("fs:default"),
            public_base_url: "file://".into(),
            max_depth: Some(32),
        }
    }
}

struct Inner {
    root: PathBuf,
    config: FsBlobStoreConfig,
    presign_key: PresignKey,
    // Serialises atomic-rename ops on the same key. The HashMap
    // grows to one entry per active key during contention and is
    // cleaned opportunistically.
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

/// Filesystem-backed blob store.
#[derive(Clone)]
pub struct FsBlobStore {
    inner: Arc<Inner>,
}

impl FsBlobStore {
    /// Open (or initialise) a store rooted at `root`, signing
    /// presigned URLs with `presign_key`.
    ///
    /// The constructor refuses to generate a `PresignKey` on the
    /// caller's behalf — see the [crate-level docs](crate) for the
    /// B3 reasoning. For tests, reach for
    /// [`PresignKey::ephemeral`].
    pub fn open(root: impl AsRef<Path>, presign_key: PresignKey) -> Result<Self, FsBlobStoreError> {
        Self::open_with_config(root, presign_key, FsBlobStoreConfig::default())
    }

    /// Same as [`FsBlobStore::open`] with a caller-supplied config.
    pub fn open_with_config(
        root: impl AsRef<Path>,
        presign_key: PresignKey,
        config: FsBlobStoreConfig,
    ) -> Result<Self, FsBlobStoreError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        if !root.is_dir() {
            return Err(FsBlobStoreError::NotADirectory(root));
        }
        Ok(Self {
            inner: Arc::new(Inner {
                root,
                config,
                presign_key,
                locks: Mutex::new(HashMap::new()),
            }),
        })
    }

    fn data_path(&self, key: &str) -> PathBuf {
        self.inner.root.join(key)
    }

    fn meta_path(&self, key: &str) -> PathBuf {
        let data = self.inner.root.join(key);
        let name = data
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        data.with_file_name(format!("{name}.meta.json"))
    }

    async fn key_lock(&self, key: &str) -> Arc<Mutex<()>> {
        let mut map = self.inner.locks.lock().await;
        map.entry(key.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Crate-internal access to the presign key, for the router.
    #[cfg(feature = "axum")]
    pub(crate) fn presign_key(&self) -> &PresignKey {
        &self.inner.presign_key
    }

    /// Crate-internal access to the root directory, for the router.
    #[cfg(feature = "axum")]
    pub(crate) fn root(&self) -> &Path {
        &self.inner.root
    }

    /// Crate-internal sidecar reader, for the router GET handler.
    #[cfg(feature = "axum")]
    pub(crate) async fn read_meta(&self, key: &str) -> Option<BlobMeta> {
        load_meta(&self.meta_path(key), &self.data_path(key))
            .await
            .ok()
    }
}

fn etag_for(bytes: &[u8]) -> Etag {
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    let hex: String = d.iter().take(16).map(|b| format!("{b:02x}")).collect();
    Etag::new(hex)
}

async fn load_sidecar(path: &Path) -> Result<Sidecar, BlobError> {
    let raw = tokio::fs::read(path).await.map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => BlobError::NotFound,
        _ => BlobError::backend(e),
    })?;
    serde_json::from_slice(&raw).map_err(BlobError::backend)
}

async fn load_meta(meta_path: &Path, data_path: &Path) -> Result<BlobMeta, BlobError> {
    let s = load_sidecar(meta_path).await?;
    let size = tokio::fs::metadata(data_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(BlobMeta::new(size, Etag::new(s.etag))
        .with_content_type(s.content_type)
        .with_cache_control(s.cache_control)
        .with_created_at(Some(s.created_at))
        .with_updated_at(Some(s.updated_at)))
}

#[async_trait]
impl BlobStore for FsBlobStore {
    fn backend_id(&self) -> &BackendId {
        &self.inner.config.backend_id
    }

    async fn put_bytes(
        &self,
        key: &BlobKey,
        bytes: Bytes,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        debug!(target: TRACE_TARGET, key = %key, bytes = bytes.len(), "put_bytes");
        let lock = self.key_lock(key.as_str()).await;
        let _guard = lock.lock().await;

        let dst = self.data_path(key.as_str());
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BlobError::backend)?;
        }

        // Conditional create via O_EXCL: stake a sentinel file
        // first so that a concurrent if_absent racer loses cleanly.
        if opts.if_absent {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dst)
            {
                Ok(_) => {
                    // We claimed the slot; remove the empty sentinel
                    // so the tempfile rename below can take its
                    // place atomically.
                    let _ = std::fs::remove_file(&dst);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(BlobError::AlreadyExists);
                }
                Err(e) => return Err(BlobError::backend(e)),
            }
        }

        let parent = dst
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let tmp = NamedTempFile::new_in(&parent).map_err(BlobError::backend)?;
        // Write via tokio for the async-shaped trait, but the
        // NamedTempFile path is sync — open the file once with
        // std, write, sync, persist.
        {
            let f: &std::fs::File = tmp.as_file();
            use std::io::Write;
            let mut bw = std::io::BufWriter::new(f);
            bw.write_all(&bytes).map_err(BlobError::backend)?;
            bw.flush().map_err(BlobError::backend)?;
        }
        tmp.as_file().sync_all().map_err(BlobError::backend)?;
        tmp.persist(&dst).map_err(|e| BlobError::backend(e.error))?;

        let now = Utc::now();
        let etag = etag_for(&bytes);
        let prior = load_sidecar(&self.meta_path(key.as_str())).await.ok();
        let sidecar = Sidecar {
            content_type: opts.content_type.clone(),
            cache_control: opts.cache_control.clone(),
            etag: etag.as_str().to_owned(),
            created_at: prior.as_ref().map(|s| s.created_at).unwrap_or(now),
            updated_at: now,
        };
        let meta_tmp = NamedTempFile::new_in(&parent).map_err(BlobError::backend)?;
        {
            use std::io::Write;
            let json = serde_json::to_vec(&sidecar).map_err(BlobError::backend)?;
            let f: &std::fs::File = meta_tmp.as_file();
            let mut bw = std::io::BufWriter::new(f);
            bw.write_all(&json).map_err(BlobError::backend)?;
            bw.flush().map_err(BlobError::backend)?;
        }
        meta_tmp.as_file().sync_all().map_err(BlobError::backend)?;
        meta_tmp
            .persist(self.meta_path(key.as_str()))
            .map_err(|e| BlobError::backend(e.error))?;

        Ok(BlobRef::mint(
            self.inner.config.backend_id.clone(),
            key.as_str().to_owned(),
            etag,
            bytes.len() as u64,
        ))
    }

    async fn put_stream(
        &self,
        key: &BlobKey,
        stream: BoxStream<'static, Result<Bytes, BlobError>>,
        opts: PutOptions,
    ) -> Result<BlobRef, BlobError> {
        // Buffer to disk via a tempfile in the destination's
        // parent — preserves atomic-rename semantics for the
        // streaming path too.
        let dst = self.data_path(key.as_str());
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(BlobError::backend)?;
        }
        let parent = dst
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let lock = self.key_lock(key.as_str()).await;
        let _guard = lock.lock().await;

        if opts.if_absent && dst.exists() {
            return Err(BlobError::AlreadyExists);
        }

        let mut total = BytesMut::new();
        let mut s = stream;
        while let Some(chunk) = s.try_next().await? {
            total.extend_from_slice(&chunk);
        }
        let bytes = total.freeze();

        let tmp = NamedTempFile::new_in(&parent).map_err(BlobError::backend)?;
        {
            use std::io::Write;
            let f: &std::fs::File = tmp.as_file();
            let mut bw = std::io::BufWriter::new(f);
            bw.write_all(&bytes).map_err(BlobError::backend)?;
            bw.flush().map_err(BlobError::backend)?;
        }
        tmp.as_file().sync_all().map_err(BlobError::backend)?;
        tmp.persist(&dst).map_err(|e| BlobError::backend(e.error))?;

        let now = Utc::now();
        let etag = etag_for(&bytes);
        let prior = load_sidecar(&self.meta_path(key.as_str())).await.ok();
        let sidecar = Sidecar {
            content_type: opts.content_type.clone(),
            cache_control: opts.cache_control.clone(),
            etag: etag.as_str().to_owned(),
            created_at: prior.as_ref().map(|s| s.created_at).unwrap_or(now),
            updated_at: now,
        };
        let meta_tmp = NamedTempFile::new_in(&parent).map_err(BlobError::backend)?;
        {
            use std::io::Write;
            let json = serde_json::to_vec(&sidecar).map_err(BlobError::backend)?;
            let f: &std::fs::File = meta_tmp.as_file();
            let mut bw = std::io::BufWriter::new(f);
            bw.write_all(&json).map_err(BlobError::backend)?;
            bw.flush().map_err(BlobError::backend)?;
        }
        meta_tmp.as_file().sync_all().map_err(BlobError::backend)?;
        meta_tmp
            .persist(self.meta_path(key.as_str()))
            .map_err(|e| BlobError::backend(e.error))?;

        Ok(BlobRef::mint(
            self.inner.config.backend_id.clone(),
            key.as_str().to_owned(),
            etag,
            bytes.len() as u64,
        ))
    }

    async fn get(
        &self,
        blob_ref: &BlobRef,
        range: Option<BlobRange>,
    ) -> Result<BoxStream<'static, Result<Bytes, BlobError>>, BlobError> {
        let path = self.data_path(blob_ref.opaque_locator());
        let mut file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => BlobError::NotFound,
                _ => BlobError::backend(e),
            })?;
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf)
            .await
            .map_err(BlobError::backend)?;
        let bytes = match range {
            Some(r) => {
                let len = buf.len() as u64;
                if r.start >= len {
                    Bytes::new()
                } else {
                    let end = r.end.min(len - 1) as usize;
                    let start = r.start as usize;
                    Bytes::from(buf[start..=end].to_vec())
                }
            }
            None => Bytes::from(buf),
        };
        Ok(stream::once(async move { Ok(bytes) }).boxed())
    }

    async fn head(&self, blob_ref: &BlobRef) -> Result<BlobMeta, BlobError> {
        let meta_path = self.meta_path(blob_ref.opaque_locator());
        let data_path = self.data_path(blob_ref.opaque_locator());
        // Bounce off the data file first so a missing blob with a
        // stale sidecar still surfaces as NotFound.
        tokio::fs::metadata(&data_path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => BlobError::NotFound,
                _ => BlobError::backend(e),
            })?;
        load_meta(&meta_path, &data_path).await
    }

    async fn delete(&self, blob_ref: &BlobRef) -> Result<(), BlobError> {
        let lock = self.key_lock(blob_ref.opaque_locator()).await;
        let _g = lock.lock().await;
        let _ = tokio::fs::remove_file(self.data_path(blob_ref.opaque_locator())).await;
        let _ = tokio::fs::remove_file(self.meta_path(blob_ref.opaque_locator())).await;
        Ok(())
    }

    async fn list(
        &self,
        prefix: Option<&BlobKey>,
        cursor: Option<&str>,
    ) -> Result<ListPage, BlobError> {
        let prefix_owned = prefix.map(|p| p.as_str().to_owned()).unwrap_or_default();
        let root = self.inner.root.clone();
        let max_depth = self.inner.config.max_depth;
        let backend_id = self.inner.config.backend_id.clone();

        // walkdir is sync; do it on a blocking thread.
        let keys = tokio::task::spawn_blocking(move || {
            let mut walker = WalkDir::new(&root);
            if let Some(d) = max_depth {
                walker = walker.max_depth(d);
            }
            let mut out: Vec<String> = Vec::new();
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let Ok(rel) = path.strip_prefix(&root) else {
                    continue;
                };
                let key = rel.to_string_lossy().into_owned();
                if key.ends_with(".meta.json") {
                    continue;
                }
                if !key.starts_with(&prefix_owned) {
                    continue;
                }
                out.push(key);
            }
            out.sort();
            out
        })
        .await
        .map_err(BlobError::backend)?;

        let start = match cursor {
            Some(c) => keys
                .iter()
                .position(|k| k.as_str() > c)
                .unwrap_or(keys.len()),
            None => 0,
        };
        const PAGE: usize = 1000;
        let end = (start + PAGE).min(keys.len());

        let mut items = Vec::with_capacity(end - start);
        for k in &keys[start..end] {
            let meta = match load_meta(&self.meta_path(k), &self.data_path(k)).await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let r = BlobRef::mint(backend_id.clone(), k.clone(), meta.etag.clone(), meta.size);
            items.push((r, meta));
        }
        let next_cursor = (end < keys.len()).then(|| keys[end - 1].clone());
        Ok(ListPage::new(items, next_cursor))
    }

    async fn approximate_usage(&self, prefix: &BlobKey) -> Result<BlobUsage, BlobError> {
        // Authoritative: walk the keyspace, skip sidecars, sum file
        // lengths via stat. `walkdir` is sync — hop to a blocking
        // pool so we do not stall the runtime on a deep tree.
        let prefix_owned = prefix.as_str().to_owned();
        let root = self.inner.root.clone();
        let max_depth = self.inner.config.max_depth;
        tokio::task::spawn_blocking(move || {
            let mut walker = WalkDir::new(&root);
            if let Some(d) = max_depth {
                walker = walker.max_depth(d);
            }
            let mut bytes: u64 = 0;
            let mut objects: u64 = 0;
            for entry in walker.into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                let Ok(rel) = path.strip_prefix(&root) else {
                    continue;
                };
                let key = rel.to_string_lossy();
                if key.ends_with(".meta.json") {
                    continue;
                }
                if !key.starts_with(&prefix_owned) {
                    continue;
                }
                if let Ok(md) = entry.metadata() {
                    bytes = bytes.saturating_add(md.len());
                    objects = objects.saturating_add(1);
                }
            }
            Ok(BlobUsage::new(bytes, objects))
        })
        .await
        .map_err(BlobError::backend)?
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
        let token = presign::sign(&self.inner.presign_key, &claim);
        let url = format!(
            "{base}{sep}token={token}",
            base = self.inner.config.public_base_url,
            sep = if self.inner.config.public_base_url.contains('?') {
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

    fn k(s: &str) -> BlobKey {
        BlobKey::new(s).unwrap()
    }

    async fn drain(s: BoxStream<'static, Result<Bytes, BlobError>>) -> Vec<u8> {
        let chunks: Vec<Bytes> = s.try_collect().await.unwrap();
        chunks.iter().flat_map(|b| b.iter().copied()).collect()
    }

    fn store() -> (FsBlobStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let s = FsBlobStore::open(dir.path(), PresignKey::ephemeral()).unwrap();
        (s, dir)
    }

    #[tokio::test]
    async fn roundtrip_persists_to_disk() {
        let (s, _dir) = store();
        let r = s
            .put_bytes(
                &k("a/b.txt"),
                Bytes::from_static(b"hello"),
                PutOptions::with_content_type("text/plain"),
            )
            .await
            .unwrap();
        let got = drain(s.get(&r, None).await.unwrap()).await;
        assert_eq!(got, b"hello");
        let meta = s.head(&r).await.unwrap();
        assert_eq!(meta.size, 5);
        assert_eq!(meta.content_type.as_deref(), Some("text/plain"));
    }

    #[tokio::test]
    async fn write_is_atomic_no_partial_files() {
        let (s, dir) = store();
        s.put_bytes(&k("o"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        s.put_bytes(&k("o"), Bytes::from_static(b"v2"), PutOptions::default())
            .await
            .unwrap();
        // No leftover tempfiles in the root.
        let stragglers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp"))
            .collect();
        assert!(stragglers.is_empty(), "leftover temp: {stragglers:?}");
    }

    #[tokio::test]
    async fn if_absent_refuses_overwrite() {
        let (s, _d) = store();
        s.put_bytes(&k("x"), Bytes::from_static(b"v1"), PutOptions::default())
            .await
            .unwrap();
        let err = s
            .put_bytes(
                &k("x"),
                Bytes::from_static(b"v2"),
                PutOptions::default().if_absent(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, BlobError::AlreadyExists));
    }

    #[tokio::test]
    async fn list_respects_prefix_and_skips_sidecars() {
        let (s, _d) = store();
        for c in ["t/7/a", "t/7/b", "t/8/a"] {
            s.put_bytes(&k(c), Bytes::from_static(b"x"), PutOptions::default())
                .await
                .unwrap();
        }
        let page = s.list(Some(&k("t/7/")), None).await.unwrap();
        assert_eq!(page.items.len(), 2);
        // No sidecar masquerading as a blob.
        for (r, _) in &page.items {
            assert!(!r.opaque_locator().ends_with(".meta.json"));
        }
    }

    #[tokio::test]
    async fn delete_is_idempotent() {
        let (s, _d) = store();
        let r = s
            .put_bytes(&k("d"), Bytes::from_static(b"x"), PutOptions::default())
            .await
            .unwrap();
        s.delete(&r).await.unwrap();
        s.delete(&r).await.unwrap();
        assert!(matches!(s.head(&r).await.unwrap_err(), BlobError::NotFound));
    }

    #[tokio::test]
    async fn approximate_usage_walks_tree_and_skips_sidecars() {
        let (s, _d) = store();
        for (key, body) in [("t/7/a", &b"aaaa"[..]), ("t/7/b", &b"bb"[..]), ("t/8/x", &b"xxxxxxxx"[..])] {
            s.put_bytes(&k(key), Bytes::copy_from_slice(body), PutOptions::default())
                .await
                .unwrap();
        }
        let u = s.approximate_usage(&k("t/7/")).await.unwrap();
        assert_eq!(u, BlobUsage::new(6, 2), "sidecar files must not count");
        let all = s.approximate_usage(&k("t/")).await.unwrap();
        assert_eq!(all, BlobUsage::new(14, 3));
    }

    #[tokio::test]
    async fn presign_uses_caller_supplied_key() {
        let dir = tempfile::tempdir().unwrap();
        let key = PresignKey::from_bytes([9u8; 32]);
        let s = FsBlobStore::open(dir.path(), key.clone()).unwrap();
        let r = s
            .put_bytes(&k("k"), Bytes::from_static(b"hi"), PutOptions::default())
            .await
            .unwrap();
        let url = s
            .presign(&r, PresignOp::Get, Duration::from_secs(60))
            .await
            .unwrap();
        let tok = url.url.split("token=").nth(1).unwrap();
        let claim = presign::verify(&key, tok).unwrap();
        assert_eq!(claim.locator, "k");

        // A different key rejects the same token — caller, not
        // engine, owns the trust root.
        let other = PresignKey::from_bytes([0u8; 32]);
        assert!(presign::verify(&other, tok).is_err());
    }
}
