//! `WindowedFetcher` — the per-bucket fetch trait per-engine impls
//! satisfy. Engine-agnostic; live in the consumer crates.

use crate::bucket::Bucket;
use crate::stitch::Stitchable;
use async_trait::async_trait;

/// Generic fetch error any engine surfaces back to the caller.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// Engine-specific failure with a human-readable description.
    #[error("windowed fetch: {0}")]
    Other(String),
}

/// Fetch one bucket's worth of data. Implementations live in the
/// per-engine crates (`starter-store-warehouse`,
/// `starter-store-postgres`, or in-memory mocks in tests).
#[async_trait]
pub trait WindowedFetcher<T: Stitchable>: Send + Sync {
    /// Fetch the rows for `bucket` (UTC half-open `[start, end)`).
    async fn fetch_bucket(&self, bucket: Bucket) -> Result<T, FetchError>;
}
