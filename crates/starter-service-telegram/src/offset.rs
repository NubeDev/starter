//! [`OffsetStore`] — where the `getUpdates` cookie lives across
//! polls.
//!
//! For v0.1 the only impl is [`InMemoryOffsetStore`]: the cookie
//! lives in a `Mutex<Option<i64>>` and resets to `None` when the
//! consumer restarts. Telegram keeps undelivered updates for 24h, so
//! the worst-case effect of restarting mid-stream is re-delivery of
//! whatever was already acked — the consumer's `EventSink` must be
//! idempotent (which the SCOPE R4 contract already implies — services
//! emit raw payloads, the consumer's domain layer decides what's a
//! duplicate).
//!
//! The trait shape is the *seam* an at-rest backend will slot into
//! later. Keeping it here, even with just one impl, means future
//! `starter-store-sqlite` / `starter-store-postgres` add-ons can land
//! without a breaking change to [`crate::TelegramBotService::new`].

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

/// Persistence seam for the `getUpdates` cookie.
///
/// `load` returns the offset to start the next poll at; `store` is
/// called after every successful batch with the new `max(update_id)
/// + 1`. Implementations must be cheap — both methods land in the hot
/// loop.
///
/// Errors are deliberately stringified (`String`) rather than typed:
/// the long-poll loop logs them and continues with the in-memory
/// fallback. A typed enum here would force every future backend
/// (sqlite, redis, …) to map onto the same shape; the SCOPE keeps the
/// SPI cheap to depend on, and offsets are not worth a new trait
/// hierarchy.
#[async_trait]
pub trait OffsetStore: Send + Sync + 'static {
    /// Read the last persisted offset.
    async fn load(&self) -> Result<Option<i64>, String>;
    /// Persist a new offset.
    async fn store(&self, offset: i64) -> Result<(), String>;
}

/// Default in-memory store. Holds the offset in a `Mutex<Option<i64>>`
/// and resets to `None` on restart. Cheap to clone — internally
/// `Arc`-shared.
#[derive(Default, Clone)]
pub struct InMemoryOffsetStore {
    inner: Arc<Mutex<Option<i64>>>,
}

impl InMemoryOffsetStore {
    /// Build an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OffsetStore for InMemoryOffsetStore {
    async fn load(&self) -> Result<Option<i64>, String> {
        Ok(*self.inner.lock().expect("offset mutex poisoned"))
    }

    async fn store(&self, offset: i64) -> Result<(), String> {
        *self.inner.lock().expect("offset mutex poisoned") = Some(offset);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trips_through_memory() {
        let store = InMemoryOffsetStore::new();
        assert_eq!(store.load().await.unwrap(), None);
        store.store(42).await.unwrap();
        assert_eq!(store.load().await.unwrap(), Some(42));
        store.store(100).await.unwrap();
        assert_eq!(store.load().await.unwrap(), Some(100));
    }
}
