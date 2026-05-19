//! [`ServiceRegistry`] — open, consumer-built collection of
//! [`Service`](super::Service) impls. Mirrors `ToolRegistry`'s shape
//! (see `starter-mcp::registry::ToolRegistry`).
//!
//! The registry owns the **single** `tokio::sync::watch::Sender<bool>`
//! and hands every spawned service a clone of the receiver via
//! [`ServiceContext::shutdown`](super::ServiceContext). No service
//! holds the sender (SCOPE R2).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::timeout;

use crate::error::Result;

use super::context::ServiceContext;
use super::handle::ServiceHandle;
use super::service::Service;
use super::sink::EventSink;

/// Default deadline `ServiceRegistry::shutdown` enforces before
/// force-aborting straggler services. Decision D3 — bumping this is a
/// SemVer-visible edit.
pub const SHUTDOWN_DEADLINE_DEFAULT: Duration = Duration::from_secs(5);

/// Per-service shutdown outcome captured in [`ShutdownReport`].
#[derive(Debug)]
pub enum ServiceShutdownOutcome {
    /// Service joined cleanly within the deadline.
    Clean,
    /// Service joined within the deadline but its loop returned an
    /// error.
    Error(crate::error::Error),
    /// Service panicked or was cancelled.
    JoinError(tokio::task::JoinError),
    /// Deadline elapsed before the join handle resolved; the registry
    /// aborted the task. Implementations that hit this are not
    /// observing `ServiceContext.shutdown` and fail SCOPE smoke
    /// test 5.
    Aborted,
}

/// Per-service shutdown record. Returned by
/// [`ServiceRegistry::shutdown`] so callers can log which services
/// drained cleanly and which had to be force-aborted.
#[derive(Debug)]
pub struct ShutdownReport {
    /// One entry per registered service, in registration order.
    pub services: Vec<(String, ServiceShutdownOutcome)>,
}

/// Mutable builder + runtime registry of [`Service`] impls.
pub struct ServiceRegistry {
    services: Vec<(String, Arc<dyn Service>)>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    handles: Vec<(String, ServiceHandle)>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    /// Empty registry. A fresh `watch::channel(false)` is created
    /// here; the sender stays inside the registry forever.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            services: Vec::new(),
            shutdown_tx: tx,
            shutdown_rx: rx,
            handles: Vec::new(),
        }
    }

    /// Register a service. Last-write-wins on duplicate names is
    /// **not** enforced — registering two services with the same
    /// `name()` will produce two distinct spawned tasks. Consumers
    /// who care should dedupe at the call site.
    pub fn register<S: Service>(mut self, service: S) -> Self {
        let name = service.name().to_string();
        self.services.push((name, Arc::new(service)));
        self
    }

    /// All registered service names, in registration order.
    pub fn names(&self) -> Vec<&str> {
        self.services.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Number of registered services.
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Whether the registry has zero registered services.
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Build a [`ServiceContext`] suitable for one service, with the
    /// registry's `shutdown` receiver and the supplied sink and
    /// metrics registry.
    pub fn context_with_sink(
        &self,
        metrics: Arc<prometheus::Registry>,
        sink: Arc<dyn EventSink>,
    ) -> ServiceContext {
        ServiceContext::new(metrics, self.shutdown_rx.clone(), sink)
    }

    /// Start every registered service, handing each a fresh
    /// `ServiceContext` built from the supplied metrics registry and
    /// (shared) sink. The returned handles are also retained inside
    /// the registry so [`shutdown`](Self::shutdown) can drive them
    /// later.
    pub async fn start_all(
        &mut self,
        metrics: Arc<prometheus::Registry>,
        sink: Arc<dyn EventSink>,
    ) -> Result<()> {
        for (name, service) in &self.services {
            let ctx = ServiceContext::new(metrics.clone(), self.shutdown_rx.clone(), sink.clone());
            let handle = service.start(ctx).await?;
            self.handles.push((name.clone(), handle));
        }
        Ok(())
    }

    /// Signal cooperative shutdown and await every started service,
    /// using the default deadline ([`SHUTDOWN_DEADLINE_DEFAULT`]).
    pub async fn shutdown(&mut self) -> ShutdownReport {
        self.shutdown_with_deadline(SHUTDOWN_DEADLINE_DEFAULT).await
    }

    /// Signal cooperative shutdown and await every started service
    /// for up to `deadline`. Services that don't observe
    /// `ServiceContext.shutdown` get force-aborted and show up in the
    /// returned [`ShutdownReport`] as
    /// [`ServiceShutdownOutcome::Aborted`].
    pub async fn shutdown_with_deadline(&mut self, deadline: Duration) -> ShutdownReport {
        // Flip the watch. Ignored if no receivers — that just means
        // no services were started.
        let _ = self.shutdown_tx.send(true);

        let mut outcomes = Vec::with_capacity(self.handles.len());
        for (name, handle) in self.handles.drain(..) {
            let ServiceHandle { join } = handle;
            let abort_handle = join.abort_handle();
            let outcome = match timeout(deadline, join).await {
                Ok(Ok(Ok(()))) => ServiceShutdownOutcome::Clean,
                Ok(Ok(Err(err))) => ServiceShutdownOutcome::Error(err),
                Ok(Err(join_err)) => ServiceShutdownOutcome::JoinError(join_err),
                Err(_) => {
                    abort_handle.abort();
                    ServiceShutdownOutcome::Aborted
                }
            };
            outcomes.push((name, outcome));
        }

        ShutdownReport { services: outcomes }
    }
}
