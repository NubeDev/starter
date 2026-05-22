//! Error surface for cache operations that can fail.
//!
//! Today only [`Cache::get_or_insert_with`](crate::Cache::get_or_insert_with)
//! produces errors — the loader closure can fail. The enum is
//! deliberately small; new variants land here as new fallible
//! operations are added (e.g. network-backed backends needing a
//! `Backend` variant).

use thiserror::Error;

/// Errors returned by fallible cache operations.
#[derive(Debug, Error)]
pub enum CacheError<E> {
    /// The loader passed to `get_or_insert_with` failed. The inner
    /// error is whatever the caller's loader returned.
    #[error("cache loader failed: {0}")]
    Loader(E),
}
