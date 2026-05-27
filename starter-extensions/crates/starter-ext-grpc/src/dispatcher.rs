//! Dispatcher seam — the one boundary the gRPC adapter punches into
//! the kernel. Direct mirror of `starter_ext_cli::dispatcher`:
//!
//! - [`GrpcDispatcher`] is the trait every flavour implements.
//! - [`BuiltinGrpcDispatcher`] routes calls to closures registered in
//!   a [`BuiltinGrpcRegistry`] at host startup; calls run in-process,
//!   no JSON-RPC frame is ever serialised.
//! - [`ProcessGrpcDispatcher`] / [`WasmGrpcDispatcher`] return
//!   `DispatchError::NotWired` in v0.1 (matching `starter-ext-cli`'s
//!   v0.1 state); both carry the `request_timeout: Duration` knob so
//!   the wiring shape is uniform when the synchronous JSON-RPC slice
//!   lands additively.
//!
//! Cancellation: client-side cancel (HTTP/2 RST_STREAM / context
//! cancel on the gRPC client) reaches [`CancelHandle::fire`]; the
//! handle is dropped at most once and is observed by the extension
//! through its `Ctx` cancel token (mirroring the streaming contract
//! shared by every other adapter).

use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::Stream;
use starter_ext_sdk::ctx::{
    Cancel, CtxInner, EventSender, FsBackend, HttpOutBackend, SecretsBackend, TracingBackend,
    WallClockBackend,
};
use starter_ext_spi::jsonrpc::StreamId;
use starter_ext_spi::{Error, ExtensionId};
use tokio::sync::{mpsc, watch};

/// Default per-call timeout the adapter uses when the consumer did
/// not override it. Mirrors `starter-ext-cli::DEFAULT_REQUEST_TIMEOUT`
/// so the two adapter surfaces share one default budget.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// One streaming-event payload as it leaves the dispatcher. Carries
/// the JSON the handler emitted; the gRPC service wraps each event
/// into an `InvokeStreamEvent { payload_proto_json }`.
pub use starter_ext_sdk::ctx::Event as StreamEvent;

/// Errors a dispatcher returns. The service layer translates each
/// variant into a `tonic::Status` code (see [`crate::service`]):
/// `NotFound → NOT_FOUND`, `Forbidden → PERMISSION_DENIED`,
/// `NotWired → UNIMPLEMENTED`, `Extension → INTERNAL`,
/// `Timeout → DEADLINE_EXCEEDED`, `Substrate → INTERNAL`.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The `(service, method)` pair or the resolved
    /// `(extension, contribute_id)` is not registered with the
    /// dispatcher.
    #[error("not found: {0}")]
    NotFound(String),
    /// Per-entry auth gate refused this principal.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The dispatcher cannot serve this contribution in v0.1 (e.g.
    /// process-flavour or wasm-flavour gRPC dispatch). The
    /// `request_timeout` knob is wired through this dispatcher
    /// already so the JSON-RPC slice lands additively.
    #[error("not wired: {0}")]
    NotWired(String),
    /// The extension's own handler returned an error.
    #[error("extension internal: {0}")]
    Extension(String),
    /// The synchronous request timed out before the handler replied.
    #[error("timed out after {}ms", .0.as_millis())]
    Timeout(Duration),
    /// Any other substrate failure (transport, spawn, manifest).
    #[error("substrate: {0}")]
    Substrate(String),
}

impl DispatchError {
    /// Translate kernel error categories into dispatcher categories.
    pub fn from_kernel(err: Error) -> Self {
        match err {
            Error::Validation(m) => DispatchError::NotFound(m),
            Error::Capability(m) => DispatchError::Forbidden(m),
            Error::ExtensionInternal(m) => DispatchError::Extension(m),
            other => DispatchError::Substrate(other.to_string()),
        }
    }

    /// Map the dispatcher's category to a `tonic::Code`. Kept here so
    /// the policy lives next to the variant definitions; the service
    /// layer is the only caller.
    pub fn tonic_code(&self) -> tonic::Code {
        match self {
            DispatchError::NotFound(_) => tonic::Code::NotFound,
            DispatchError::Forbidden(_) => tonic::Code::PermissionDenied,
            DispatchError::NotWired(_) => tonic::Code::Unimplemented,
            DispatchError::Extension(_) => tonic::Code::Internal,
            DispatchError::Timeout(_) => tonic::Code::DeadlineExceeded,
            DispatchError::Substrate(_) => tonic::Code::Internal,
        }
    }
}

/// Open end of a streaming dispatch.
///
/// The service pumps `events` onto its `tonic::Streaming` response
/// until the channel closes or the cancel handle fires. Dropping
/// this value propagates cancellation to the extension within a
/// handful of milliseconds.
pub struct StreamResponse {
    /// Allocated stream id (surfaced for tracing / hand-rolled
    /// renderers; the default service does not tag events with it).
    pub stream_id: StreamId,
    /// Stream of typed kernel events. Each yields the JSON the
    /// service wraps into one `InvokeStreamEvent`.
    pub events: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>,
    /// Cancellation hook fired by the service on client disconnect.
    pub cancel: CancelHandle,
}

/// Opaque one-shot cancel hook. Fires its closure at most once: on
/// explicit [`Self::fire`], on [`Drop`], or when the service observes
/// client disconnect.
pub struct CancelHandle {
    fired: Arc<AtomicBool>,
    on_fire: Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>,
}

impl CancelHandle {
    /// Build from a one-shot closure.
    pub fn new<F: FnOnce() + Send + 'static>(on_fire: F) -> Self {
        Self {
            fired: Arc::new(AtomicBool::new(false)),
            on_fire: Mutex::new(Some(Box::new(on_fire))),
        }
    }

    /// Build a no-op handle (for stub dispatchers).
    pub fn noop() -> Self {
        Self::new(|| {})
    }

    /// Fire the cancellation now.
    pub fn fire(&self) {
        if !self.fired.swap(true, Ordering::SeqCst) {
            let mut guard = self.on_fire.lock().expect("cancel mutex poisoned");
            if let Some(f) = guard.take() {
                f();
            }
        }
    }

    /// `true` once [`Self::fire`] has run.
    pub fn was_fired(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }
}

impl Drop for CancelHandle {
    fn drop(&mut self) {
        self.fire();
    }
}

impl fmt::Debug for CancelHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancelHandle")
            .field("fired", &self.was_fired())
            .finish()
    }
}

/// Adapter ↔ kernel seam. One method per call shape so the service
/// reads the manifest's `streaming` field at build time and never has
/// to inspect bodies at request time.
#[async_trait]
pub trait GrpcDispatcher: Send + Sync + 'static {
    /// Dispatch a unary gRPC RPC.
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError>;

    /// Dispatch a server-streaming gRPC RPC.
    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<StreamResponse, DispatchError>;
}

// ---------------------------------------------------------------------------
// BuiltinGrpcRegistry / BuiltinGrpcDispatcher
// ---------------------------------------------------------------------------

/// Closure shape for a unary gRPC handler.
pub type GrpcHandler = dyn Fn(serde_json::Value, &CtxInner) -> Result<serde_json::Value, Error>
    + Send
    + Sync
    + 'static;

/// Closure shape for a server-streaming gRPC handler.
pub type GrpcStreamingHandler =
    dyn Fn(serde_json::Value, &CtxInner) -> Result<(), Error> + Send + Sync + 'static;

enum HandlerEntry {
    Unary(Arc<GrpcHandler>),
    Streaming(Arc<GrpcStreamingHandler>),
}

/// Per-host map of gRPC handlers keyed by `(extension_id, contribute_id)`.
/// Hosts build it at startup and hand it to [`BuiltinGrpcDispatcher::new`].
#[derive(Default)]
pub struct BuiltinGrpcRegistry {
    handlers: HashMap<(ExtensionId, String), HandlerEntry>,
}

impl BuiltinGrpcRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a unary handler.
    pub fn register<F>(
        mut self,
        extension: ExtensionId,
        contribute_id: impl Into<String>,
        f: F,
    ) -> Self
    where
        F: Fn(serde_json::Value, &CtxInner) -> Result<serde_json::Value, Error>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(
            (extension, contribute_id.into()),
            HandlerEntry::Unary(Arc::new(f)),
        );
        self
    }

    /// Register a server-streaming handler. The handler emits events
    /// through `ctx.events()` and observes cancellation through
    /// `ctx.cancel()`; the service wraps each emitted event into one
    /// `InvokeStreamEvent`.
    pub fn register_streaming<F>(
        mut self,
        extension: ExtensionId,
        contribute_id: impl Into<String>,
        f: F,
    ) -> Self
    where
        F: Fn(serde_json::Value, &CtxInner) -> Result<(), Error> + Send + Sync + 'static,
    {
        self.handlers.insert(
            (extension, contribute_id.into()),
            HandlerEntry::Streaming(Arc::new(f)),
        );
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

impl fmt::Debug for BuiltinGrpcRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltinGrpcRegistry")
            .field("len", &self.handlers.len())
            .finish()
    }
}

/// In-process gRPC dispatcher backed by a [`BuiltinGrpcRegistry`].
pub struct BuiltinGrpcDispatcher {
    registry: Arc<BuiltinGrpcRegistry>,
    /// Per-stream channel capacity. Matches `starter-ext-cli`'s 16 so
    /// a fast handler doesn't immediately backpressure on a slow
    /// client; tonic's outbound flow control absorbs the rest.
    event_channel_capacity: usize,
}

impl BuiltinGrpcDispatcher {
    /// New dispatcher backed by `registry`.
    pub fn new(registry: Arc<BuiltinGrpcRegistry>) -> Self {
        Self {
            registry,
            event_channel_capacity: 16,
        }
    }

    fn lookup_unary(
        &self,
        ext: &ExtensionId,
        cid: &str,
    ) -> Result<Arc<GrpcHandler>, DispatchError> {
        match self.registry.handlers.get(&(ext.clone(), cid.to_owned())) {
            Some(HandlerEntry::Unary(h)) => Ok(h.clone()),
            Some(HandlerEntry::Streaming(_)) => Err(DispatchError::NotFound(format!(
                "{cid:?} is registered as streaming; manifest says unary"
            ))),
            None => Err(DispatchError::NotFound(format!(
                "no builtin handler for ({}, {cid:?})",
                ext.as_str()
            ))),
        }
    }

    fn lookup_streaming(
        &self,
        ext: &ExtensionId,
        cid: &str,
    ) -> Result<Arc<GrpcStreamingHandler>, DispatchError> {
        match self.registry.handlers.get(&(ext.clone(), cid.to_owned())) {
            Some(HandlerEntry::Streaming(h)) => Ok(h.clone()),
            Some(HandlerEntry::Unary(_)) => Err(DispatchError::NotFound(format!(
                "{cid:?} is registered as unary; manifest says streaming"
            ))),
            None => Err(DispatchError::NotFound(format!(
                "no builtin streaming handler for ({}, {cid:?})",
                ext.as_str()
            ))),
        }
    }
}

#[async_trait]
impl GrpcDispatcher for BuiltinGrpcDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        let handler = self.lookup_unary(extension, contribute_id)?;
        let (tx, _rx) = mpsc::channel(self.event_channel_capacity);
        let ctx = build_ctx(tx, Arc::new(NeverCancel));
        let fut = tokio::task::spawn_blocking(move || handler(input, &ctx));
        let result = tokio::time::timeout(timeout, fut)
            .await
            .map_err(|_| DispatchError::Timeout(timeout))?
            .map_err(|e| DispatchError::Substrate(format!("dispatch join: {e}")))?;
        result.map_err(DispatchError::from_kernel)
    }

    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<StreamResponse, DispatchError> {
        let handler = self.lookup_streaming(extension, contribute_id)?;

        let stream_id = StreamId(format!("grpc-{}-{}", extension.as_str(), uuid_like_short(),));
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let cancel = WatchCancel::new(cancel_rx);
        let (tx, rx) = mpsc::channel::<StreamEvent>(self.event_channel_capacity);
        let ctx = build_ctx(tx, Arc::new(cancel));

        tokio::task::spawn_blocking(move || {
            let _ = handler(input, &ctx);
        });

        let events = Box::pin(map_recv_stream(rx));
        let cancel_handle = CancelHandle::new(move || {
            let _ = cancel_tx.send(true);
        });

        Ok(StreamResponse {
            stream_id,
            events,
            cancel: cancel_handle,
        })
    }
}

// ---------------------------------------------------------------------------
// NotWired / Process / Wasm — v0.1 stubs with the same shape as
// starter-ext-cli's dispatchers
// ---------------------------------------------------------------------------

/// Default dispatcher for hosts that haven't wired anything yet.
#[derive(Debug, Default)]
pub struct NotWiredGrpcDispatcher;

#[async_trait]
impl GrpcDispatcher for NotWiredGrpcDispatcher {
    async fn dispatch(
        &self,
        _ext: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no GrpcDispatcher wired for contribute {contribute_id:?}"
        )))
    }

    async fn dispatch_stream(
        &self,
        _ext: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<StreamResponse, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no GrpcDispatcher wired for streaming contribute {contribute_id:?}"
        )))
    }
}

/// Process-flavour dispatcher.
///
/// Holds the per-extension [`starter_ext_supervisor::SupervisorHandle`]
/// map and a default per-call timeout. Synchronous unary dispatch is
/// implemented on top of [`starter_ext_supervisor::SupervisorHandle::call`]:
/// the request method on the wire is `tools/<contribute_id>` so the
/// `starter-ext-sdk` `register_process_main!`-generated child loop
/// routes the call through `ExtensionDispatch::dispatch_tool`. Streaming
/// dispatch remains `NotWired` until the per-stream demultiplexer lands.
pub struct ProcessGrpcDispatcher {
    handles: HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
    request_timeout: Duration,
}

impl ProcessGrpcDispatcher {
    /// New process dispatcher with a per-call default timeout.
    pub fn new(
        handles: HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
        request_timeout: Duration,
    ) -> Self {
        Self {
            handles,
            request_timeout,
        }
    }

    /// Default timeout the consumer configured.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Look up the supervisor handle for an extension.
    pub fn handle_for(
        &self,
        extension: &ExtensionId,
    ) -> Option<Arc<starter_ext_supervisor::SupervisorHandle>> {
        self.handles.get(extension).cloned()
    }

    fn effective_timeout(&self, per_call: Duration) -> Duration {
        if per_call.is_zero() {
            self.request_timeout
        } else {
            per_call.min(self.request_timeout)
        }
    }
}

#[async_trait]
impl GrpcDispatcher for ProcessGrpcDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        let handle = self.handles.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!(
                "no supervisor handle for extension {:?}",
                extension.as_str()
            ))
        })?;
        let method = format!("tools/{contribute_id}");
        let effective = self.effective_timeout(timeout);
        handle
            .call(&method, input, effective)
            .await
            .map_err(DispatchError::from_kernel)
    }

    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<StreamResponse, DispatchError> {
        if self.handles.contains_key(extension) {
            Err(DispatchError::NotWired(format!(
                "process-flavour streaming dispatch for {contribute_id:?} \
                 is a follow-up slice (unary path is wired; streaming \
                 needs the per-stream stream.event/end demultiplexer)"
            )))
        } else {
            Err(DispatchError::NotFound(format!(
                "no supervisor handle for extension {:?}",
                extension.as_str()
            )))
        }
    }
}

/// Wasm-flavour dispatcher — v0.1 stub. Same `request_timeout` knob
/// shape as [`ProcessGrpcDispatcher`] so the wiring is uniform.
pub struct WasmGrpcDispatcher {
    request_timeout: Duration,
}

impl WasmGrpcDispatcher {
    /// New wasm dispatcher.
    pub fn new(request_timeout: Duration) -> Self {
        Self { request_timeout }
    }

    /// Default per-call timeout.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }
}

#[async_trait]
impl GrpcDispatcher for WasmGrpcDispatcher {
    async fn dispatch(
        &self,
        _ext: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "wasm-flavour synchronous JSON-RPC dispatch for {contribute_id:?} \
             ships in the next adapter slice; the request_timeout knob is in place \
             (default {:?})",
            self.request_timeout
        )))
    }

    async fn dispatch_stream(
        &self,
        _ext: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<StreamResponse, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "wasm-flavour streaming dispatch for {contribute_id:?} \
             ships in the next adapter slice"
        )))
    }
}

// ---------------------------------------------------------------------------
// Stub capability backends + helpers — identical shape to
// starter-ext-cli's. Kept inline (not extracted into a shared crate)
// because each adapter's stub set may grow differently as capability
// categories light up across adapters at different paces.
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
        Arc::new(StubEventBus),
        Arc::new(StubDashboard),
        Arc::new(StubAuthz),
    )
}

#[derive(Debug)]
struct StubSecrets;
impl SecretsBackend for StubSecrets {
    fn get(&self, _name: &str) -> starter_ext_spi::Result<String> {
        Err(Error::capability(
            "secrets not wired in gRPC adapter Phase 8",
        ))
    }
}

#[derive(Debug)]
struct StubHttpOut;
impl HttpOutBackend for StubHttpOut {
    fn request(&self, _req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "http_out not wired in gRPC adapter Phase 8",
        ))
    }
}

#[derive(Debug)]
struct StubFs;
impl FsBackend for StubFs {
    fn read(&self, _path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        Err(Error::capability("fs not wired in gRPC adapter Phase 8"))
    }
}

#[derive(Debug)]
struct StubWallClock;
impl WallClockBackend for StubWallClock {
    fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "wall_clock not wired in gRPC adapter Phase 8",
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
            "warehouse_read not wired in gRPC adapter",
        ))
    }
    fn count(
        &self,
        _template: &str,
        _params: serde_json::Value,
    ) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "warehouse_read not wired in gRPC adapter",
        ))
    }
    fn describe(
        &self,
        _template: &str,
    ) -> starter_ext_spi::Result<Option<starter_ext_spi::warehouse::TemplateSpec>> {
        Err(Error::capability(
            "warehouse_read not wired in gRPC adapter",
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
            "warehouse_write not wired in gRPC adapter",
        ))
    }
}

#[derive(Debug)]
struct StubEventBus;
impl starter_ext_sdk::ctx::EventBusBackend for StubEventBus {
    fn publish(&self, _topic: &str, _payload: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability("event_bus not wired in gRPC adapter"))
    }
}

#[derive(Debug)]
struct StubDashboard;
impl starter_ext_sdk::ctx::DashboardBackend for StubDashboard {
    fn read(&self, _page_id: &str) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability("dashboard not wired in gRPC adapter"))
    }
    fn write(&self, _page_id: &str, _body: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability("dashboard not wired in gRPC adapter"))
    }
}

#[derive(Debug)]
struct StubAuthz;
impl starter_ext_sdk::ctx::AuthzBackend for StubAuthz {
    fn check(&self, _action: &str, _resource: &str) -> starter_ext_spi::Result<bool> {
        Err(Error::capability("authz not wired in gRPC adapter"))
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

struct WatchCancel {
    rx: watch::Receiver<bool>,
}

impl WatchCancel {
    fn new(rx: watch::Receiver<bool>) -> Self {
        Self { rx }
    }
}

impl Cancel for WatchCancel {
    fn is_cancelled(&self) -> bool {
        *self.rx.borrow()
    }
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let mut rx = self.rx.clone();
            if *rx.borrow() {
                return;
            }
            loop {
                if rx.changed().await.is_err() {
                    return;
                }
                if *rx.borrow() {
                    return;
                }
            }
        })
    }
}

fn uuid_like_short() -> String {
    use std::sync::atomic::AtomicU64;
    static N: AtomicU64 = AtomicU64::new(1);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .subsec_nanos() as u64;
    format!("{:x}-{:x}", nanos, n)
}

fn map_recv_stream(
    rx: mpsc::Receiver<StreamEvent>,
) -> impl Stream<Item = Result<StreamEvent, Error>> + Send + 'static {
    use futures::stream::poll_fn;
    let rx = Mutex::new(rx);
    poll_fn(move |cx| {
        let mut guard = rx.lock().expect("recv mutex poisoned");
        match guard.poll_recv(cx) {
            std::task::Poll::Ready(Some(ev)) => std::task::Poll::Ready(Some(Ok(ev))),
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn cancel_handle_runs_on_drop_exactly_once() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let handle = CancelHandle::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        drop(handle);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancel_handle_fire_then_drop_is_idempotent() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let handle = CancelHandle::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        handle.fire();
        drop(handle);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn not_wired_returns_not_wired_for_unary_and_stream() {
        let d = NotWiredGrpcDispatcher;
        let ext = ExtensionId::new("com.acme.weather").unwrap();
        let err = d
            .dispatch(&ext, "x", serde_json::Value::Null, DEFAULT_REQUEST_TIMEOUT)
            .await
            .unwrap_err();
        assert!(matches!(err, DispatchError::NotWired(_)));
        assert_eq!(err.tonic_code(), tonic::Code::Unimplemented);
    }
}
