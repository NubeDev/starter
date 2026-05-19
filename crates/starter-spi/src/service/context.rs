//! [`ServiceContext`] — the bag of cross-cutting handles every
//! [`Service`](super::Service) is given at startup.

use std::sync::Arc;

use tokio::sync::watch;

use super::sink::EventSink;

/// Cross-cutting context handed to every [`Service::start`](super::Service::start)
/// call.
///
/// Marked `#[non_exhaustive]` so adding a new field (a `RestartPolicy`,
/// a `tracing::Span`, a `CancellationToken`, …) is additive, not
/// breaking. Construct via [`ServiceContext::new`] or, if you have a
/// [`ServiceRegistry`](super::ServiceRegistry), via
/// `ServiceRegistry::context_with_sink`.
#[non_exhaustive]
pub struct ServiceContext {
    /// Shared prometheus registry. Provider crates register their own
    /// metrics here (SCOPE R7); the SPI does **not** ship any helper —
    /// raw `register_*` calls are how it gets done.
    pub metrics: Arc<prometheus::Registry>,

    /// Cooperative-shutdown signal. The owning
    /// [`ServiceRegistry`](super::ServiceRegistry) holds the matching
    /// `watch::Sender<bool>`; services watch this receiver and exit
    /// when it flips to `true` (SCOPE R2 — no service holds the
    /// sender).
    pub shutdown: watch::Receiver<bool>,

    /// Where the service publishes events. Typed as `Arc<dyn
    /// EventSink>` so different services in the same binary can share
    /// or differ on dispatch shape (a fan-out, a broadcast channel, a
    /// test double).
    pub sink: Arc<dyn EventSink>,
}

impl ServiceContext {
    /// Build a context from raw parts. Most callers go through
    /// [`ServiceRegistry::context_with_sink`](super::ServiceRegistry::context_with_sink)
    /// instead so they don't have to wire the watch receiver
    /// themselves.
    pub fn new(
        metrics: Arc<prometheus::Registry>,
        shutdown: watch::Receiver<bool>,
        sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            metrics,
            shutdown,
            sink,
        }
    }
}
