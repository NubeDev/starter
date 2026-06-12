//! Dispatcher seam between the periodic scheduler and the kernel.
//!
//! Mirrors the shape used by `starter-ext-cli::dispatcher` and
//! `starter-ext-server::rest::dispatcher`:
//!
//! - [`WorkerDispatcher`] — one trait method (`run`) per call; the
//!   scheduler invokes it on every `next_due` firing.
//! - [`BuiltinWorkerDispatcher`] — registry of in-process handler
//!   closures keyed by `(extension_id, worker_id)`. Runs on the tokio
//!   blocking pool so a slow handler does not park the scheduler
//!   runtime.
//! - [`ProcessWorkerDispatcher`] / [`WasmWorkerDispatcher`] — v0.1
//!   stubs that return `WorkerError::NotWired`. Both ship the same
//!   `request_timeout` knob so the wiring shape is uniform when the
//!   synchronous JSON-RPC slice lands.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use starter_ext_sdk::ctx::{
    Cancel, CtxInner, EventSender, FsBackend, HttpOutBackend, SecretsBackend, TracingBackend,
    WallClockBackend,
};
use starter_ext_spi::{Error, ExtensionId};
use tokio::sync::mpsc;

/// Default request timeout the scheduler applies to a single worker
/// run. A periodic worker that exceeds this is treated as a failure
/// (which then feeds the `on_error` policy). Picked larger than a
/// human-noticeable HTTP timeout so brief downstream slowness does
/// not flap the worker into Stopped.
pub const DEFAULT_WORKER_TIMEOUT: Duration = Duration::from_secs(60);

/// Failure modes from one dispatch.
///
/// Mapped onto [`crate::WorkerState::last_error`] by the scheduler.
/// Distinct variants exist so `Stopped` can be reached deterministically
/// from `NotFound` / `Forbidden` (configuration errors that retrying
/// will never fix), not just from a generic "the extension said no".
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// No handler is registered for `(extension_id, worker_id)`.
    /// Configuration error — the scheduler treats it as immediately
    /// `Stopped` rather than burning through `max_attempts`.
    #[error("not found: {0}")]
    NotFound(String),

    /// The dispatcher refused the call at the auth layer the adapter
    /// owns (system-principal gates, future per-entry `auth` checks).
    /// Treated as `Stopped` immediately.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// The dispatcher cannot serve this contribute kind in v0.1
    /// (process/wasm-flavour scheduler dispatch).
    #[error("not wired: {0}")]
    NotWired(String),

    /// The handler returned an error. The scheduler increments
    /// `attempt`, records `last_error`, and applies `on_error`.
    #[error("extension internal: {0}")]
    Extension(String),

    /// The dispatch exceeded `request_timeout`. Treated as a regular
    /// failure (counts against `max_attempts`).
    #[error("timed out after {}ms", .0.as_millis())]
    Timeout(Duration),

    /// Any other substrate failure — transport, spawn, manifest.
    #[error("substrate: {0}")]
    Substrate(String),
}

impl WorkerError {
    /// Translate from the kernel error categories.
    pub fn from_kernel(err: Error) -> Self {
        match err {
            Error::Validation(m) => WorkerError::NotFound(m),
            Error::Capability(m) => WorkerError::Forbidden(m),
            Error::ExtensionInternal(m) => WorkerError::Extension(m),
            other => WorkerError::Substrate(other.to_string()),
        }
    }

    /// `true` when the failure is a configuration error that no
    /// number of retries can fix. The scheduler short-circuits to
    /// [`crate::WorkerStatus::Stopped`] on these.
    pub fn is_fatal_config(&self) -> bool {
        matches!(self, WorkerError::NotFound(_) | WorkerError::Forbidden(_))
    }
}

/// Closure shape for a builtin worker handler. Receives the per-call
/// `Ctx`; the worker's "input" is implicit (the scheduler fires it on
/// a timer, not from operator input). Returns `Ok(())` on a successful
/// run; `Err` triggers `on_error` handling.
pub type WorkerHandler = dyn Fn(&CtxInner) -> Result<(), Error> + Send + Sync + 'static;

/// The adapter ↔ kernel seam.
#[async_trait]
pub trait WorkerDispatcher: Send + Sync + 'static {
    /// Run one tick of a single worker. The dispatcher is responsible
    /// for honouring `timeout`; the scheduler maps a `Timeout` back
    /// onto `on_error`.
    async fn run(
        &self,
        extension: &ExtensionId,
        worker_id: &str,
        timeout: Duration,
    ) -> Result<(), WorkerError>;
}

// ---------------------------------------------------------------------------
// BuiltinWorkerRegistry / BuiltinWorkerDispatcher
// ---------------------------------------------------------------------------

/// Per-host map of worker handlers keyed by `(extension_id, worker_id)`.
#[derive(Default)]
pub struct BuiltinWorkerRegistry {
    handlers: HashMap<(ExtensionId, String), Arc<WorkerHandler>>,
}

impl BuiltinWorkerRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a worker handler. Returns the registry for builder
    /// chaining.
    pub fn register<F>(mut self, extension: ExtensionId, worker_id: impl Into<String>, f: F) -> Self
    where
        F: Fn(&CtxInner) -> Result<(), Error> + Send + Sync + 'static,
    {
        self.handlers
            .insert((extension, worker_id.into()), Arc::new(f));
        self
    }

    /// Number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// `true` if no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl std::fmt::Debug for BuiltinWorkerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinWorkerRegistry")
            .field("len", &self.handlers.len())
            .finish()
    }
}

/// Routes scheduler ticks to handlers in a [`BuiltinWorkerRegistry`].
pub struct BuiltinWorkerDispatcher {
    registry: Arc<BuiltinWorkerRegistry>,
    event_channel_capacity: usize,
}

impl BuiltinWorkerDispatcher {
    /// New builtin dispatcher.
    pub fn new(registry: Arc<BuiltinWorkerRegistry>) -> Self {
        Self {
            registry,
            event_channel_capacity: 8,
        }
    }
}

#[async_trait]
impl WorkerDispatcher for BuiltinWorkerDispatcher {
    async fn run(
        &self,
        extension: &ExtensionId,
        worker_id: &str,
        timeout: Duration,
    ) -> Result<(), WorkerError> {
        let handler = self
            .registry
            .handlers
            .get(&(extension.clone(), worker_id.to_owned()))
            .cloned()
            .ok_or_else(|| {
                WorkerError::NotFound(format!(
                    "no builtin handler registered for ({}, {worker_id:?})",
                    extension.as_str()
                ))
            })?;

        let (tx, _rx) = mpsc::channel(self.event_channel_capacity);
        let ctx = build_ctx(tx, Arc::new(NeverCancel));

        let fut = tokio::task::spawn_blocking(move || handler(&ctx));
        let result = tokio::time::timeout(timeout, fut)
            .await
            .map_err(|_| WorkerError::Timeout(timeout))?
            .map_err(|e| WorkerError::Substrate(format!("dispatch join: {e}")))?;
        result.map_err(WorkerError::from_kernel)
    }
}

// ---------------------------------------------------------------------------
// NotWired / Process / Wasm stubs
// ---------------------------------------------------------------------------

/// Default dispatcher for hosts that have not wired anything yet.
/// Every tick returns `WorkerError::NotWired` (which the scheduler
/// records as a sticky configuration error).
#[derive(Debug, Default)]
pub struct NotWiredWorkerDispatcher;

#[async_trait]
impl WorkerDispatcher for NotWiredWorkerDispatcher {
    async fn run(
        &self,
        _extension: &ExtensionId,
        worker_id: &str,
        _timeout: Duration,
    ) -> Result<(), WorkerError> {
        Err(WorkerError::NotWired(format!(
            "no WorkerDispatcher wired for worker {worker_id:?}"
        )))
    }
}

/// Process-flavour dispatcher — v0.1 stub.
///
/// Carries the `request_timeout` knob so the wiring shape is uniform
/// with [`BuiltinWorkerDispatcher`]; the synchronous JSON-RPC body
/// fills in additively without touching the scheduler.
pub struct ProcessWorkerDispatcher {
    request_timeout: Duration,
}

impl ProcessWorkerDispatcher {
    /// New process dispatcher with a configurable request timeout.
    pub fn new(request_timeout: Duration) -> Self {
        Self { request_timeout }
    }

    /// Configured default request timeout.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }
}

#[async_trait]
impl WorkerDispatcher for ProcessWorkerDispatcher {
    async fn run(
        &self,
        _extension: &ExtensionId,
        worker_id: &str,
        _timeout: Duration,
    ) -> Result<(), WorkerError> {
        Err(WorkerError::NotWired(format!(
            "process-flavour worker dispatch for {worker_id:?} ships in the next adapter slice; \
             request_timeout in place (default {:?})",
            self.request_timeout
        )))
    }
}

/// Wasm-flavour dispatcher — v0.1 stub.
pub struct WasmWorkerDispatcher {
    request_timeout: Duration,
}

impl WasmWorkerDispatcher {
    /// New wasm dispatcher with a configurable request timeout.
    pub fn new(request_timeout: Duration) -> Self {
        Self { request_timeout }
    }

    /// Configured default request timeout.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }
}

#[async_trait]
impl WorkerDispatcher for WasmWorkerDispatcher {
    async fn run(
        &self,
        _extension: &ExtensionId,
        worker_id: &str,
        _timeout: Duration,
    ) -> Result<(), WorkerError> {
        Err(WorkerError::NotWired(format!(
            "wasm-flavour worker dispatch for {worker_id:?} ships in the next adapter slice; \
             request_timeout in place (default {:?})",
            self.request_timeout
        )))
    }
}

// ---------------------------------------------------------------------------
// Stub capability backends
// ---------------------------------------------------------------------------

fn build_ctx(events: EventSender, cancel: Arc<dyn Cancel>) -> CtxInner {
    CtxInner::new(
        events,
        cancel,
        Arc::new(StubSecrets),
        Arc::new(StubHttpOut),
        Arc::new(StubFs),
        Arc::new(StubWallClock),
        Arc::new(StubTracing),
        Arc::new(StubWarehouseRead),
        Arc::new(StubWarehouseWrite),
        Arc::new(StubDatasource),
        Arc::new(StubEventBus),
        Arc::new(StubExtensionCall),
        Arc::new(StubDashboard),
        Arc::new(StubAuthz),
    )
}

#[derive(Debug)]
struct StubSecrets;
impl SecretsBackend for StubSecrets {
    fn get(&self, _name: &str) -> starter_ext_spi::Result<String> {
        Err(Error::capability(
            "secrets not wired in workers adapter Phase 7",
        ))
    }
}

#[derive(Debug)]
struct StubHttpOut;
impl HttpOutBackend for StubHttpOut {
    fn request(&self, _req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "http_out not wired in workers adapter Phase 7",
        ))
    }
}

#[derive(Debug)]
struct StubFs;
impl FsBackend for StubFs {
    fn read(&self, _path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        Err(Error::capability("fs not wired in workers adapter Phase 7"))
    }
}

#[derive(Debug)]
struct StubWallClock;
impl WallClockBackend for StubWallClock {
    fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "wall_clock not wired in workers adapter Phase 7",
        ))
    }
}

#[derive(Debug)]
struct StubTracing;
impl TracingBackend for StubTracing {
    fn event(&self, _level: &str, _msg: &str, _fields: serde_json::Value) {}
}

#[derive(Debug)]
struct StubWarehouseRead;
impl starter_ext_sdk::ctx::WarehouseReadBackend for StubWarehouseRead {
    fn query(
        &self,
        _template: &str,
        _params: serde_json::Value,
    ) -> starter_ext_spi::Result<Vec<starter_ext_spi::warehouse::Row>> {
        Err(Error::capability(
            "warehouse_read not wired in workers adapter",
        ))
    }
    fn count(&self, _template: &str, _params: serde_json::Value) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "warehouse_read not wired in workers adapter",
        ))
    }
    fn describe(
        &self,
        _template: &str,
    ) -> starter_ext_spi::Result<Option<starter_ext_spi::warehouse::TemplateSpec>> {
        Err(Error::capability(
            "warehouse_read not wired in workers adapter",
        ))
    }
}

#[derive(Debug)]
struct StubWarehouseWrite;
impl starter_ext_sdk::ctx::WarehouseWriteBackend for StubWarehouseWrite {
    fn insert(
        &self,
        _table: &str,
        _rows: Vec<starter_ext_spi::warehouse::Row>,
    ) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "warehouse_write not wired in workers adapter",
        ))
    }
}

#[derive(Debug)]
struct StubDatasource;
impl starter_ext_sdk::ctx::DatasourceBackend for StubDatasource {
    fn query(
        &self,
        _id: &str,
        _sql: &str,
        _params: Vec<serde_json::Value>,
    ) -> starter_ext_spi::Result<Vec<starter_ext_spi::warehouse::Row>> {
        Err(Error::capability("datasource not wired in workers adapter"))
    }
    fn execute(
        &self,
        _id: &str,
        _stmt: &str,
        _params: Vec<serde_json::Value>,
    ) -> starter_ext_spi::Result<u64> {
        Err(Error::capability("datasource not wired in workers adapter"))
    }
}

#[derive(Debug)]
struct StubEventBus;
impl starter_ext_sdk::ctx::EventBusBackend for StubEventBus {
    fn publish(&self, _topic: &str, _payload: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability("event_bus not wired in workers adapter"))
    }
}

#[derive(Debug)]
struct StubExtensionCall;
impl starter_ext_sdk::ctx::ExtensionCallBackend for StubExtensionCall {
    fn call(
        &self,
        _extension_id: &str,
        _provided_id: &str,
        _input: serde_json::Value,
    ) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "extension_call not wired in workers adapter",
        ))
    }
}

#[derive(Debug)]
struct StubDashboard;
impl starter_ext_sdk::ctx::DashboardBackend for StubDashboard {
    fn read(&self, _page_id: &str) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability("dashboard not wired in workers adapter"))
    }
    fn write(&self, _page_id: &str, _body: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability("dashboard not wired in workers adapter"))
    }
}

#[derive(Debug)]
struct StubAuthz;
impl starter_ext_sdk::ctx::AuthzBackend for StubAuthz {
    fn check(&self, _action: &str, _resource: &str) -> starter_ext_spi::Result<bool> {
        Err(Error::capability("authz not wired in workers adapter"))
    }
}

#[derive(Debug, Default)]
struct NeverCancel;
impl Cancel for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn builtin_dispatcher_runs_a_handler() {
        let ext = ExtensionId::new("com.acme.x").unwrap();
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let reg = BuiltinWorkerRegistry::new().register(ext.clone(), "com.acme.x.w", move |_| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let d = BuiltinWorkerDispatcher::new(Arc::new(reg));
        d.run(&ext, "com.acme.x.w", Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn builtin_dispatcher_times_out_long_handler() {
        let ext = ExtensionId::new("com.acme.x").unwrap();
        let reg = BuiltinWorkerRegistry::new().register(ext.clone(), "com.acme.x.slow", |_| {
            std::thread::sleep(Duration::from_millis(300));
            Ok(())
        });
        let d = BuiltinWorkerDispatcher::new(Arc::new(reg));
        let r = d
            .run(&ext, "com.acme.x.slow", Duration::from_millis(30))
            .await;
        assert!(matches!(r, Err(WorkerError::Timeout(_))));
    }

    #[tokio::test]
    async fn not_wired_dispatcher_returns_not_wired() {
        let d = NotWiredWorkerDispatcher;
        let id = ExtensionId::new("com.acme.x").unwrap();
        let r = d.run(&id, "com.acme.x.w", Duration::from_secs(1)).await;
        assert!(matches!(r, Err(WorkerError::NotWired(_))));
    }

    #[test]
    fn not_found_is_fatal_config() {
        assert!(WorkerError::NotFound("x".into()).is_fatal_config());
        assert!(WorkerError::Forbidden("x".into()).is_fatal_config());
        assert!(!WorkerError::Extension("x".into()).is_fatal_config());
        assert!(!WorkerError::Timeout(Duration::from_secs(1)).is_fatal_config());
    }
}
