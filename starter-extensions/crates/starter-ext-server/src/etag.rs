//! ETag cache for UI bundle serving.
//!
//! `GET /extensions/<id>/ui/*` serves static files out of an extension's
//! bundle directory. We hand each response a strong ETag derived from a
//! SHA-256 of the file's bytes so a Module-Federation host can cache
//! chunks aggressively across hot reloads. Computing the digest on every
//! request would be wasteful — files in a deployed bundle don't change
//! at runtime — so we memoise by canonical path + mtime + size. If any
//! of those three change, we recompute.

use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

/// One cached ETag entry. The mtime + size act as a cheap freshness
/// check before we trust the cached digest.
#[derive(Debug, Clone)]
struct Entry {
    etag: String,
    mtime: SystemTime,
    size: u64,
}

/// Thread-safe ETag cache, sized to grow with the bundle's chunk count
/// (a few dozen for a typical Module-Federation remote). No explicit
/// eviction — entries that go stale just get recomputed in-place.
#[derive(Debug, Default)]
pub(crate) struct EtagCache {
    inner: RwLock<HashMap<PathBuf, Entry>>,
}

impl EtagCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Get an ETag for `path`, recomputing if mtime or size has changed.
    /// Returns `Ok((etag, bytes))` so the caller can stream the body
    /// without re-reading from disk.
    pub(crate) async fn etag_and_bytes(
        &self,
        path: &Path,
    ) -> std::io::Result<(String, Vec<u8>)> {
        let meta = tokio::fs::metadata(path).await?;
        if !meta.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path is not a regular file",
            ));
        }
        let mtime = mtime_of(&meta);
        let size = meta.len();

        // Fast path: cache hit + still-fresh metadata. We always have to
        // read the bytes (the handler streams the body), but we skip the
        // SHA-256 work and re-use the stored digest. The clone-and-drop
        // pattern is deliberate — holding a `RwLockReadGuard` across the
        // `tokio::fs::read` await would make the handler future `!Send`.
        let cached = {
            self.inner
                .read()
                .expect("etag cache poisoned")
                .get(path)
                .cloned()
        };
        if let Some(entry) = cached {
            if entry.mtime == mtime && entry.size == size {
                let bytes = tokio::fs::read(path).await?;
                return Ok((entry.etag, bytes));
            }
        }

        // Slow path: digest the bytes, install, return.
        let bytes = tokio::fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hasher.finalize();
        let etag = format!("\"{}\"", hex::encode(digest));
        self.inner
            .write()
            .expect("etag cache poisoned")
            .insert(
                path.to_path_buf(),
                Entry {
                    etag: etag.clone(),
                    mtime,
                    size,
                },
            );
        Ok((etag, bytes))
    }
}

fn mtime_of(meta: &Metadata) -> SystemTime {
    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}
