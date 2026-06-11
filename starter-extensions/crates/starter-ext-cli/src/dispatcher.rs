//! Dispatcher seam — the one boundary the CLI adapter punches into
//! the kernel.
//!
//! Mirrors the shape of `starter-ext-server::rest::dispatcher`:
//!
//! - [`CliDispatcher`] is the trait every flavour implements.
//! - [`BuiltinCliDispatcher`] runs in-process via a
//!   [`BuiltinCliRegistry`] the host populates at startup.
//! - [`ProcessCliDispatcher`] / [`WasmCliDispatcher`] return
//!   `DispatchError::NotWired` in v0.1; both accept the configurable
//!   `request_timeout: Duration` so the synchronous JSON-RPC slice
//!   lands additively in the next iteration without changing call
//!   sites.
//!
//! Non-streaming dispatch is `async fn dispatch(timeout)`; streaming
//! dispatch is `async fn dispatch_stream(timeout) -> StreamResponse`
//! and the response carries a [`CancelHandle`] the adapter fires on
//! `SIGINT` or on dropping the response, mapping to the kernel's
//! `stream.cancel` notification (SCOPE post-R13).

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

/// Default request timeout the adapter uses when the host did not
/// override it. Picked to match the supervisor's default health timeout
/// order-of-magnitude, big enough that a slow `clap` parse on the
/// extension side never trips it. The CLI adapter passes this on to
/// [`CliDispatcher::dispatch`] / [`CliDispatcher::dispatch_stream`].
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// One streaming-event payload as it leaves the dispatcher. Carries the
/// `stream_id` the initial dispatch allocated; the CLI renderer turns
/// each payload into one line on stdout.
pub use starter_ext_sdk::ctx::Event as StreamEvent;

/// Distinct from [`starter_ext_spi::Error`] because the CLI adapter
/// translates each variant into a different exit code / stderr shape
/// (see [`crate::command::ExtensionSubcommand::run`]) — and that mapping
/// is the adapter's job, not the kernel's.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The contribute id is not registered with the dispatcher (e.g.
    /// the extension is `Failed`, missing from the
    /// [`BuiltinCliRegistry`], …). Translates to exit code 2 with a
    /// "no such command" message.
    #[error("not found: {0}")]
    NotFound(String),

    /// The dispatcher refused the call for capability / auth reasons
    /// it owns at its layer (the manifest's `auth: { require_role,
    /// require_scope }` did not match the current principal). The
    /// adapter never lets the extension's handler see a forbidden
    /// invocation. Exit code 3.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// The dispatcher cannot serve this contribute kind in v0.1 (e.g.
    /// process-flavour or wasm-flavour CLI dispatch). Exit code 4.
    /// The configurable `request_timeout` knob is wired through this
    /// dispatcher already, so consumers can size their host-side
    /// budget today and the actual JSON-RPC wiring lands additively.
    #[error("not wired: {0}")]
    NotWired(String),

    /// The extension's own handler returned an error. Exit code 1
    /// with the extension's message on stderr; by R5 the adapter
    /// surfaces this as an application error, not a substrate crash.
    #[error("extension internal: {0}")]
    Extension(String),

    /// The synchronous JSON-RPC request to the child timed out before a
    /// response landed. Exit code 5.
    #[error("timed out after {}ms", .0.as_millis())]
    Timeout(Duration),

    /// Any other substrate failure — transport, spawn, manifest. Exit
    /// code 70 (sysexits' `EX_SOFTWARE`).
    #[error("substrate: {0}")]
    Substrate(String),
}

impl DispatchError {
    /// Translate from the kernel error categories.
    pub fn from_kernel(err: Error) -> Self {
        match err {
            Error::Validation(m) => DispatchError::NotFound(m),
            Error::Capability(m) => DispatchError::Forbidden(m),
            Error::ExtensionInternal(m) => DispatchError::Extension(m),
            other => DispatchError::Substrate(other.to_string()),
        }
    }

    /// Sysexits-style exit code the binary should return after this
    /// dispatch failure.
    pub fn exit_code(&self) -> u8 {
        match self {
            DispatchError::Extension(_) => 1,
            DispatchError::NotFound(_) => 2,
            DispatchError::Forbidden(_) => 3,
            DispatchError::NotWired(_) => 4,
            DispatchError::Timeout(_) => 5,
            DispatchError::Substrate(_) => 70,
        }
    }
}

/// The open end of a streaming dispatch.
///
/// The adapter owns the stream until the handler emits its final event
/// (the channel closes) or the user hits `Ctrl-C` (the
/// [`Self::cancel`] handle fires). Dropping this value (which happens
/// automatically when the subcommand returns) **must** propagate
/// cancellation to the extension within a few hundred milliseconds —
/// the SIGINT-becomes-stream.cancel guarantee the brief calls out.
pub struct StreamResponse {
    /// The allocated stream id. Surfaced so a hand-rolled renderer can
    /// tag every line with the same id (the default renderer just emits
    /// the payload).
    pub stream_id: StreamId,
    /// Stream of typed kernel events. Each yields the payload the
    /// renderer turns into one stdout line.
    pub events: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>,
    /// Cancellation hook the adapter fires on `SIGINT` or on response
    /// drop. Implementors translate this into the kernel's
    /// `stream.cancel` notification (for process / wasm) or into
    /// firing the per-call `Cancel` handle (for builtin).
    pub cancel: CancelHandle,
}

/// Opaque cancel hook. Fires its inner function exactly once, on the
/// first of: explicit [`Self::fire`], [`Drop`], or terminal `SIGINT`
/// (the CLI adapter wires the latter at command-run time).
pub struct CancelHandle {
    fired: Arc<AtomicBool>,
    on_fire: Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>,
}

impl CancelHandle {
    /// Build from a one-shot closure. The closure runs at most once.
    pub fn new<F: FnOnce() + Send + 'static>(on_fire: F) -> Self {
        Self {
            fired: Arc::new(AtomicBool::new(false)),
            on_fire: Mutex::new(Some(Box::new(on_fire))),
        }
    }

    /// Build a no-op handle. Useful for stub dispatchers that don't
    /// hold any state to cancel.
    pub fn noop() -> Self {
        Self::new(|| {})
    }

    /// Fire the cancellation now. Subsequent calls (including the
    /// `Drop`) are no-ops — cancellation is idempotent and one-shot.
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

/// The adapter ↔ kernel seam. One method per call shape so the adapter
/// reads the manifest's `streaming:` mode at build time and never has
/// to inspect bodies at request time to decide which path to take.
///
/// Every method accepts a `timeout: Duration`. For the builtin
/// dispatcher the timeout is observed as a tokio `timeout()` wrapper
/// around the handler closure; for process / wasm flavours it is
/// passed straight through to the synchronous JSON-RPC dispatch (when
/// that slice lands).
#[async_trait]
pub trait CliDispatcher: Send + Sync + 'static {
    /// Dispatch a non-streaming subcommand. The dispatcher owns the
    /// lifetime of the call; the adapter prints the returned value
    /// as a single JSON document on stdout.
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError>;

    /// Open a streaming dispatch. The returned [`StreamResponse`] is
    /// the adapter's read end; line-delimited JSON renderer pumps it
    /// onto stdout until the channel closes or the cancel handle
    /// fires.
    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<StreamResponse, DispatchError>;
}

// ---------------------------------------------------------------------------
// Closure types the host registers in the BuiltinCliRegistry
// ---------------------------------------------------------------------------

/// Closure shape for a non-streaming CLI handler. Receives the parsed
/// args (JSON object) and the per-call `Ctx` (capabilities + event
/// sender + cancel). Synchronous body; runs on tokio's blocking pool
/// so a slow handler does not stall the adapter's runtime.
pub type CliHandler = dyn Fn(serde_json::Value, &CtxInner) -> Result<serde_json::Value, Error>
    + Send
    + Sync
    + 'static;

/// Closure shape for a streaming CLI handler. Same args + Ctx as
/// [`CliHandler`]; the handler emits events through `ctx.events()` and
/// polls `ctx.cancel().is_cancelled()` to honor SIGINT-driven cancel.
/// Returns `Ok(())` on clean end-of-stream; `Err` aborts the stream
/// and exits non-zero.
pub type CliStreamingHandler =
    dyn Fn(serde_json::Value, &CtxInner) -> Result<(), Error> + Send + Sync + 'static;

/// Per-host map of CLI handlers keyed by `(extension_id, cli_id)`.
///
/// The proc-macro-generated `*ToolHandlers` trait covers the
/// `contributes.tools` surface; CLI handlers register here separately
/// in v0.1 to avoid widening the per-extension trait surface. Hosts
/// build the registry once at startup and hand it to
/// [`BuiltinCliDispatcher::new`].
#[derive(Default)]
pub struct BuiltinCliRegistry {
    handlers: HashMap<(ExtensionId, String), HandlerEntry>,
}

enum HandlerEntry {
    NonStreaming(Arc<CliHandler>),
    Streaming(Arc<CliStreamingHandler>),
}

impl BuiltinCliRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a non-streaming handler. Returns the registry for
    /// builder chaining.
    pub fn register<F>(mut self, extension: ExtensionId, cli_id: impl Into<String>, f: F) -> Self
    where
        F: Fn(serde_json::Value, &CtxInner) -> Result<serde_json::Value, Error>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(
            (extension, cli_id.into()),
            HandlerEntry::NonStreaming(Arc::new(f)),
        );
        self
    }

    /// Register a streaming handler. The handler emits events through
    /// `ctx.events()`; the adapter renders each one as a stdout line.
    pub fn register_streaming<F>(
        mut self,
        extension: ExtensionId,
        cli_id: impl Into<String>,
        f: F,
    ) -> Self
    where
        F: Fn(serde_json::Value, &CtxInner) -> Result<(), Error> + Send + Sync + 'static,
    {
        self.handlers.insert(
            (extension, cli_id.into()),
            HandlerEntry::Streaming(Arc::new(f)),
        );
        self
    }

    /// Number of registered handlers (test helper).
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// `true` if no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl fmt::Debug for BuiltinCliRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuiltinCliRegistry")
            .field("len", &self.handlers.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// BuiltinCliDispatcher
// ---------------------------------------------------------------------------

/// Routes calls to handlers registered in a [`BuiltinCliRegistry`]. The
/// builtin path stays entirely in-process; no JSON-RPC frame is ever
/// serialised.
pub struct BuiltinCliDispatcher {
    registry: Arc<BuiltinCliRegistry>,
    /// Per-stream channel capacity. 16 keeps a fast handler from
    /// immediately backpressuring on a slow tty; the adapter's stdout
    /// renderer drains as fast as the terminal allows.
    event_channel_capacity: usize,
}

impl BuiltinCliDispatcher {
    /// New builtin dispatcher.
    pub fn new(registry: Arc<BuiltinCliRegistry>) -> Self {
        Self {
            registry,
            event_channel_capacity: 16,
        }
    }

    fn lookup_non_streaming(
        &self,
        extension: &ExtensionId,
        cli_id: &str,
    ) -> Result<Arc<CliHandler>, DispatchError> {
        match self
            .registry
            .handlers
            .get(&(extension.clone(), cli_id.to_owned()))
        {
            Some(HandlerEntry::NonStreaming(h)) => Ok(h.clone()),
            Some(HandlerEntry::Streaming(_)) => Err(DispatchError::NotFound(format!(
                "cli {cli_id:?} is registered as streaming; manifest says non-streaming"
            ))),
            None => Err(DispatchError::NotFound(format!(
                "no builtin handler registered for ({}, {cli_id:?}) — did the host call \
                 BuiltinCliRegistry::register?",
                extension.as_str()
            ))),
        }
    }

    fn lookup_streaming(
        &self,
        extension: &ExtensionId,
        cli_id: &str,
    ) -> Result<Arc<CliStreamingHandler>, DispatchError> {
        match self
            .registry
            .handlers
            .get(&(extension.clone(), cli_id.to_owned()))
        {
            Some(HandlerEntry::Streaming(h)) => Ok(h.clone()),
            Some(HandlerEntry::NonStreaming(_)) => Err(DispatchError::NotFound(format!(
                "cli {cli_id:?} is registered as non-streaming; manifest says streaming=stdout"
            ))),
            None => Err(DispatchError::NotFound(format!(
                "no builtin streaming handler registered for ({}, {cli_id:?})",
                extension.as_str()
            ))),
        }
    }
}

#[async_trait]
impl CliDispatcher for BuiltinCliDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        let handler = self.lookup_non_streaming(extension, contribute_id)?;
        // Stub event channel + never-cancel cancel; non-streaming
        // handlers may ignore both. The bound on the channel is fine
        // because nothing reads from it.
        let (tx, _rx) = mpsc::channel(self.event_channel_capacity);
        let ctx = build_ctx(tx, Arc::new(NeverCancel));

        // Run the (sync) handler on the blocking pool so a long
        // computation does not park the async runtime.
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

        let stream_id = StreamId(format!("cli-{}-{}", extension.as_str(), uuid_like_short(),));
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let cancel = WatchCancel::new(cancel_rx);
        let (tx, rx) = mpsc::channel::<StreamEvent>(self.event_channel_capacity);
        let ctx = build_ctx(tx, Arc::new(cancel));

        // Spawn the handler. Dropping the EventSender on return is the
        // implicit `stream.end`; the receiver sees `None` and the
        // renderer winds down.
        tokio::task::spawn_blocking(move || {
            let _ = handler(input, &ctx);
        });

        let events = Box::pin(map_recv_stream(rx));
        let cancel_handle = CancelHandle::new(move || {
            // Watch send is sync; ignore the result — if the receiver
            // is already gone the handler has wound down anyway.
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
// NotWiredCliDispatcher / ProcessCliDispatcher / WasmCliDispatcher
// ---------------------------------------------------------------------------

/// Default dispatcher for hosts that haven't wired anything yet. Every
/// call returns `DispatchError::NotWired`.
#[derive(Debug, Default)]
pub struct NotWiredCliDispatcher;

#[async_trait]
impl CliDispatcher for NotWiredCliDispatcher {
    async fn dispatch(
        &self,
        _extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<serde_json::Value, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no CliDispatcher wired for contribute {contribute_id:?}"
        )))
    }

    async fn dispatch_stream(
        &self,
        _extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _timeout: Duration,
    ) -> Result<StreamResponse, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no CliDispatcher wired for streaming contribute {contribute_id:?}"
        )))
    }
}

/// Process-flavour dispatcher.
///
/// Holds the per-extension [`starter_ext_supervisor::SupervisorHandle`]
/// map and a default request timeout. Synchronous unary dispatch is
/// implemented on top of [`starter_ext_supervisor::SupervisorHandle::call`]:
/// the request method on the wire is `tools/<contribute_id>` so the
/// `starter-ext-sdk` `register_process_main!`-generated child loop
/// routes the call through `ExtensionDispatch::dispatch_tool`. Streaming
/// dispatch remains a follow-up slice (the streaming sub-protocol needs
/// its own per-stream demultiplexer).
pub struct ProcessCliDispatcher {
    handles: HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
    request_timeout: Duration,
}

impl ProcessCliDispatcher {
    /// New process dispatcher. `request_timeout` is the fallback the
    /// adapter uses when the consumer did not supply a per-call value;
    /// the actual dispatch picks `min(per_call, default)`.
    pub fn new(
        handles: HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
        request_timeout: Duration,
    ) -> Self {
        Self {
            handles,
            request_timeout,
        }
    }

    /// Read the configured default request timeout. Surfaced for the
    /// adapter's `--timeout` flag rendering.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Look up the supervisor handle for an extension. `Some` only if
    /// the consumer wired a handle for that id.
    pub fn handle_for(
        &self,
        extension: &ExtensionId,
    ) -> Option<Arc<starter_ext_supervisor::SupervisorHandle>> {
        self.handles.get(extension).cloned()
    }

    /// Pick the effective timeout for a call. A zero per-call value
    /// means "use the dispatcher default"; otherwise the smaller of
    /// the two wins so neither the caller nor the host configuration
    /// can extend the other's bound.
    fn effective_timeout(&self, per_call: Duration) -> Duration {
        if per_call.is_zero() {
            self.request_timeout
        } else {
            per_call.min(self.request_timeout)
        }
    }
}

#[async_trait]
impl CliDispatcher for ProcessCliDispatcher {
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
                 is a follow-up slice (the unary path is wired; streaming \
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

/// Wasm-flavour dispatcher — v0.1 stub.
///
/// Carries the same `request_timeout` knob as
/// [`ProcessCliDispatcher`] so the wiring shape is uniform. The body
/// fills in once the wasm host (Stage 11) acquires a synchronous
/// dispatch entry-point for CLI contributions.
pub struct WasmCliDispatcher {
    request_timeout: Duration,
}

impl WasmCliDispatcher {
    /// New wasm dispatcher with a configurable request timeout.
    pub fn new(request_timeout: Duration) -> Self {
        Self { request_timeout }
    }

    /// Read the configured default request timeout.
    pub fn default_request_timeout(&self) -> Duration {
        self.request_timeout
    }
}

#[async_trait]
impl CliDispatcher for WasmCliDispatcher {
    async fn dispatch(
        &self,
        _extension: &ExtensionId,
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
        _extension: &ExtensionId,
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
// Internals: stub capability backends, cancel adapters, mpsc→Stream helper
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
        Arc::new(StubDashboard),
        Arc::new(StubAuthz),
    )
}

#[derive(Debug)]
struct StubSecrets;
impl SecretsBackend for StubSecrets {
    fn get(&self, _name: &str) -> starter_ext_spi::Result<String> {
        Err(Error::capability(
            "secrets not wired in CLI adapter Phase 6",
        ))
    }
}

#[derive(Debug)]
struct StubHttpOut;
impl HttpOutBackend for StubHttpOut {
    fn request(&self, _req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "http_out not wired in CLI adapter Phase 6",
        ))
    }
}

#[derive(Debug)]
struct StubFs;
impl FsBackend for StubFs {
    fn read(&self, _path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        Err(Error::capability("fs not wired in CLI adapter Phase 6"))
    }
}

#[derive(Debug)]
struct StubWallClock;
impl WallClockBackend for StubWallClock {
    fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "wall_clock not wired in CLI adapter Phase 6",
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
            "warehouse_read not wired in CLI adapter Phase 6",
        ))
    }
    fn count(&self, _template: &str, _params: serde_json::Value) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "warehouse_read not wired in CLI adapter Phase 6",
        ))
    }
    fn describe(
        &self,
        _template: &str,
    ) -> starter_ext_spi::Result<Option<starter_ext_spi::warehouse::TemplateSpec>> {
        Err(Error::capability(
            "warehouse_read not wired in CLI adapter Phase 6",
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
            "warehouse_write not wired in CLI adapter Phase 6",
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
        Err(Error::capability("datasource not wired in CLI adapter Phase 6"))
    }
    fn execute(
        &self,
        _id: &str,
        _stmt: &str,
        _params: Vec<serde_json::Value>,
    ) -> starter_ext_spi::Result<u64> {
        Err(Error::capability("datasource not wired in CLI adapter Phase 6"))
    }
}

#[derive(Debug)]
struct StubEventBus;
impl starter_ext_sdk::ctx::EventBusBackend for StubEventBus {
    fn publish(&self, _topic: &str, _payload: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability(
            "event_bus not wired in CLI adapter Phase 6",
        ))
    }
}

#[derive(Debug)]
struct StubDashboard;
impl starter_ext_sdk::ctx::DashboardBackend for StubDashboard {
    fn read(&self, _page_id: &str) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "dashboard not wired in CLI adapter Phase 6",
        ))
    }
    fn write(&self, _page_id: &str, _body: serde_json::Value) -> starter_ext_spi::Result<()> {
        Err(Error::capability(
            "dashboard not wired in CLI adapter Phase 6",
        ))
    }
}

#[derive(Debug)]
struct StubAuthz;
impl starter_ext_sdk::ctx::AuthzBackend for StubAuthz {
    fn check(&self, _action: &str, _resource: &str) -> starter_ext_spi::Result<bool> {
        Err(Error::capability("authz not wired in CLI adapter Phase 6"))
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

/// Compact pseudo-unique short string for stream ids. Not crypto-strong;
/// adapters do not depend on the value beyond "different per call".
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

/// Map an mpsc receiver into a `Stream<Item = Result<StreamEvent, Error>>`.
/// We don't pull in `tokio-stream` for one wrapper.
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
    async fn not_wired_dispatcher_returns_not_wired() {
        let d = NotWiredCliDispatcher;
        let id = ExtensionId::new("com.acme.x").unwrap();
        let r = d
            .dispatch(
                &id,
                "com.acme.x.greet",
                serde_json::json!({}),
                Duration::from_secs(1),
            )
            .await;
        assert!(matches!(r, Err(DispatchError::NotWired(_))));
    }

    #[tokio::test]
    async fn builtin_dispatcher_round_trips_a_handler() {
        let ext = ExtensionId::new("com.acme.x").unwrap();
        let reg =
            BuiltinCliRegistry::new()
                .register(ext.clone(), "com.acme.x.echo", |params, _ctx| Ok(params));
        let d = BuiltinCliDispatcher::new(Arc::new(reg));
        let r = d
            .dispatch(
                &ext,
                "com.acme.x.echo",
                serde_json::json!({"k": 1}),
                Duration::from_secs(5),
            )
            .await
            .unwrap();
        assert_eq!(r, serde_json::json!({"k": 1}));
    }

    #[tokio::test]
    async fn builtin_dispatcher_times_out_long_handler() {
        let ext = ExtensionId::new("com.acme.x").unwrap();
        let reg = BuiltinCliRegistry::new().register(ext.clone(), "com.acme.x.slow", |_p, _ctx| {
            std::thread::sleep(Duration::from_millis(500));
            Ok(serde_json::json!(null))
        });
        let d = BuiltinCliDispatcher::new(Arc::new(reg));
        let r = d
            .dispatch(
                &ext,
                "com.acme.x.slow",
                serde_json::json!({}),
                Duration::from_millis(50),
            )
            .await;
        assert!(matches!(r, Err(DispatchError::Timeout(_))));
    }

    #[tokio::test]
    async fn dispatch_error_exit_codes_are_stable() {
        assert_eq!(DispatchError::Extension("e".into()).exit_code(), 1);
        assert_eq!(DispatchError::NotFound("e".into()).exit_code(), 2);
        assert_eq!(DispatchError::Forbidden("e".into()).exit_code(), 3);
        assert_eq!(DispatchError::NotWired("e".into()).exit_code(), 4);
        assert_eq!(
            DispatchError::Timeout(Duration::from_secs(1)).exit_code(),
            5
        );
        assert_eq!(DispatchError::Substrate("e".into()).exit_code(), 70);
    }
}
