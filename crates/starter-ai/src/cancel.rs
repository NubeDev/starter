//! `TokenCancel` — concrete `Cancel` impl wrapping
//! `tokio_util::sync::CancellationToken`.

use starter_spi::ai::Cancel;
use tokio_util::sync::CancellationToken;

/// `Cancel` impl backed by a `CancellationToken`. Cheap to clone via
/// the inner token's clone.
#[derive(Debug, Clone, Default)]
pub struct TokenCancel {
    inner: CancellationToken,
}

impl TokenCancel {
    /// Build a fresh cancellation handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap an existing `CancellationToken`.
    pub fn from_token(token: CancellationToken) -> Self {
        Self { inner: token }
    }

    /// Borrow the underlying token (useful when an SDK wants the
    /// concrete type).
    pub fn token(&self) -> &CancellationToken {
        &self.inner
    }

    /// Trip cancellation.
    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl Cancel for TokenCancel {
    fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled()
    }

    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(self.inner.cancelled())
    }
}
