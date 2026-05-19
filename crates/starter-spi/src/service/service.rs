//! The [`Service`] trait. Long-running counterpart to
//! [`crate::tool::Tool`].

use async_trait::async_trait;

use crate::error::Result;

use super::context::ServiceContext;
use super::handle::ServiceHandle;

/// A long-running background worker.
///
/// Implementations spawn whatever task they need inside `start` and
/// hand the [`tokio::task::JoinHandle`] back via [`ServiceHandle`]. The
/// registry observes that join handle; failures are recorded but
/// **not** auto-restarted (SCOPE R9).
///
/// Cooperative shutdown: the impl owns a clone of
/// `ctx.shutdown` and exits when the watch flips to `true`.
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Stable identifier for tracing / metrics labels. Should be
    /// kebab-case and globally unique within a consumer binary
    /// (e.g. `"slack-socket-mode"`).
    fn name(&self) -> &'static str;

    /// Start the service. The returned `JoinHandle` resolves when the
    /// service exits, either because `ctx.shutdown` flipped to `true`
    /// or because the underlying loop returned an error.
    async fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle>;
}
