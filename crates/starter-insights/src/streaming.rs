//! `StreamingDatasetRows` — the heavy-end of the `DatasetRows`
//! trait object (Insights SCOPE D1).
//!
//! `starter-spi` ships [`starter_spi::insights::VecDatasetRows`] for
//! packs returning tiny evidence rows or small datasets (rough cap:
//! ~10k rows / ~1MB). Packs that need to stream larger datasets
//! depend on `starter-insights`, which ships this chunked impl.
//!
//! The implementation is intentionally minimal: a `Vec<Vec<Value>>`
//! whose outer `Vec` is the chunk list and whose inner `Vec`s are
//! bounded by a per-chunk row cap. `snapshot()` materialises the
//! whole stream by concatenating chunks; downstream consumers that
//! want chunked iteration call [`StreamingDatasetRows::chunks`].
//!
//! A future revision will back this with a SQLite cursor /
//! `starter-store-postgres` server-side cursor; the trait surface
//! (`DatasetRows`) is the seam, so swapping the backend does not
//! ripple into rule-pack code.

use serde_json::Value;
use starter_spi::insights::DatasetRows;
use std::sync::Mutex;

/// Default rows-per-chunk cap. Sized so a single chunk fits in the
/// `VecDatasetRows`-equivalent budget (~1MB at ~100 bytes/row).
pub const DEFAULT_CHUNK_ROWS: usize = 10_000;

/// Chunked `DatasetRows` impl for streaming larger datasets.
///
/// Construct via [`StreamingDatasetRows::from_chunks`] (already-chunked
/// caller) or [`StreamingDatasetRows::from_rows`] (auto-chunked at
/// [`DEFAULT_CHUNK_ROWS`]).
#[derive(Debug)]
pub struct StreamingDatasetRows {
    chunks: Mutex<Vec<Vec<Value>>>,
    total: usize,
}

impl StreamingDatasetRows {
    /// Build from pre-chunked rows. Each inner `Vec` is one chunk.
    pub fn from_chunks(chunks: Vec<Vec<Value>>) -> Self {
        let total = chunks.iter().map(|c| c.len()).sum();
        Self {
            chunks: Mutex::new(chunks),
            total,
        }
    }

    /// Build from a flat row stream; auto-chunks at
    /// [`DEFAULT_CHUNK_ROWS`].
    pub fn from_rows<I: IntoIterator<Item = Value>>(rows: I) -> Self {
        Self::from_rows_with_chunk(rows, DEFAULT_CHUNK_ROWS)
    }

    /// Build from a flat row stream with a caller-chosen chunk size.
    pub fn from_rows_with_chunk<I: IntoIterator<Item = Value>>(rows: I, chunk_rows: usize) -> Self {
        let chunk_rows = chunk_rows.max(1);
        let mut chunks: Vec<Vec<Value>> = Vec::new();
        let mut current: Vec<Value> = Vec::with_capacity(chunk_rows);
        let mut total = 0usize;
        for row in rows {
            current.push(row);
            total += 1;
            if current.len() >= chunk_rows {
                chunks.push(std::mem::replace(
                    &mut current,
                    Vec::with_capacity(chunk_rows),
                ));
            }
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        Self {
            chunks: Mutex::new(chunks),
            total,
        }
    }

    /// Borrow the chunk list (clones the inner `Vec` to avoid lock
    /// contention bleed-through). Useful for chunked downstream
    /// consumers that don't want a flat snapshot.
    pub fn chunks(&self) -> Vec<Vec<Value>> {
        self.chunks
            .lock()
            .expect("StreamingDatasetRows poisoned")
            .clone()
    }

    /// Number of chunks (not rows).
    pub fn chunk_count(&self) -> usize {
        self.chunks
            .lock()
            .expect("StreamingDatasetRows poisoned")
            .len()
    }
}

impl DatasetRows for StreamingDatasetRows {
    fn snapshot(&self) -> Vec<Value> {
        let chunks = self.chunks.lock().expect("StreamingDatasetRows poisoned");
        let mut flat = Vec::with_capacity(self.total);
        for c in chunks.iter() {
            flat.extend_from_slice(c);
        }
        flat
    }

    fn len(&self) -> usize {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn auto_chunks_at_default_cap() {
        let rows: Vec<Value> = (0..25_000).map(|i| json!({ "i": i })).collect();
        let s = StreamingDatasetRows::from_rows(rows);
        assert_eq!(s.len(), 25_000);
        // 25_000 / 10_000 = 3 chunks (2 full + 1 partial).
        assert_eq!(s.chunk_count(), 3);
        assert_eq!(s.snapshot().len(), 25_000);
    }

    #[test]
    fn custom_chunk_size_honoured() {
        let rows: Vec<Value> = (0..7).map(|i| json!({ "i": i })).collect();
        let s = StreamingDatasetRows::from_rows_with_chunk(rows, 3);
        assert_eq!(s.chunk_count(), 3); // 3 + 3 + 1
    }

    #[test]
    fn empty_stream_has_no_chunks() {
        let s = StreamingDatasetRows::from_rows(Vec::<Value>::new());
        assert!(s.is_empty());
        assert_eq!(s.chunk_count(), 0);
    }
}
