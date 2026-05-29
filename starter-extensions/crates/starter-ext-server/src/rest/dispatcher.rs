//! Dispatcher seam — the one boundary the REST adapter punches into the
//! kernel.
//!
//! Every contributed REST route + every contributed tool ends up calling
//! [`RestDispatcher::dispatch`] (non-streaming) or
//! [`RestDispatcher::dispatch_stream`] (streaming). Two implementations
//! ship in v0.1:
//!
//! - [`BuiltinRestDispatcher`] — routes calls into a
//!   `starter-ext-sdk::builtin::BuiltinTable`. Streaming handlers emit
//!   events through the SDK's per-call `EventSender`; the adapter reads
//!   them off the matching receiver and renders SSE / NDJSON frames.
//!   Client disconnect fires a [`Cancel`] handle the extension polls,
//!   matching the post-R13 `stream.cancel` semantics from inside the
//!   host's address space.
//! - [`NotWiredDispatcher`] — default for non-builtin records. Returns
//!   `503 Service Unavailable` ("REST dispatch for runtime kind …
//!   ships in the next adapter slice"). This keeps the wiring shape
//!   uniform so the next slice (process / wasm dispatch) is additive.
//!
//! The trait is split (`dispatch` vs `dispatch_stream`) so the adapter
//! can pick the call shape from the manifest's `streaming:` mode at
//! build time. Mixing the two on one entry would force the
//! dispatcher to inspect every request — the brief makes streaming a
//! declared property of the entry, so the trait reflects that.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::Stream;
use starter_ext_host::ExtensionRegistry;
use starter_ext_sdk::builtin::BuiltinTable;
use starter_ext_sdk::ctx::{
    Cancel, CtxInner, FsBackend, HttpOutBackend, SecretsBackend, TracingBackend,
    WallClockBackend,
};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::jsonrpc::{StreamId, StreamNotification};
use starter_ext_spi::{Error, ExtensionId, RuntimeKind};
use tokio::sync::{mpsc, watch};
use tracing::{debug, warn};

use crate::capabilities::{CapabilityFactory, StubCapabilityFactory};
use crate::rest::cache::{caller_scope, dispatch_base_key, DispatcherCache};

/// One streaming-event payload as it leaves the dispatcher. Carries the
/// `stream_id` the initial dispatch allocated; the renderer (SSE or
/// NDJSON) decides how to frame the payload.
pub use starter_ext_sdk::ctx::Event as StreamEvent;

/// The HTTP status the REST adapter should serve for a dispatch failure.
///
/// Distinct from `starter_ext_spi::Error` because the adapter has to
/// pick a status code (`400` for invalid input, `404` for missing
/// extension, `503` for "not wired in v0.1") and a body shape, and that
/// decision is the adapter's job — the kernel error stays transport-
/// agnostic.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// The contribute id is not registered with the dispatcher (e.g.
    /// the extension is `Failed`, missing from the BuiltinTable, …).
    /// Maps to `404 Not Found`.
    #[error("not found: {0}")]
    NotFound(String),

    /// The dispatcher refused the call for capability / auth reasons it
    /// owns at its layer. Maps to `403 Forbidden`.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// The dispatcher cannot serve this contribute kind in v0.1 (e.g.
    /// process-flavour REST dispatch). Maps to `503 Service
    /// Unavailable`.
    #[error("not wired: {0}")]
    NotWired(String),

    /// The extension's own handler failed. Maps to `500 Internal Server
    /// Error` carrying the extension's message — by R5 the adapter
    /// surfaces this as an application error, not a substrate crash.
    #[error("extension internal: {0}")]
    Extension(String),

    /// Any other substrate failure — transport, spawn, manifest.
    /// Maps to `500 Internal Server Error`.
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
}

/// The open end of a streaming dispatch.
///
/// The adapter owns the stream until the client disconnects or the
/// extension calls `stream.end`. Dropping this value (which happens
/// automatically when the HTTP response is dropped) **must** propagate
/// cancellation to the extension within a few hundred milliseconds —
/// see the `Streaming-response-cancels-promptly` smoke test in
/// `tests/rest_streaming.rs`.
pub struct StreamResponse {
    /// The allocated stream id (used as the SSE `id:` field and the
    /// `stream_id` carried on every kernel `stream.event` /
    /// `stream.end` / `stream.cancel` notification).
    pub stream_id: StreamId,
    /// Stream of typed kernel events. Each yields the payload the
    /// renderer turns into one SSE frame / NDJSON line. Errors yielded
    /// in-stream map to `stream.error` shape; the adapter forwards them
    /// to the client per its rendering mode.
    pub events: Pin<Box<dyn Stream<Item = Result<StreamEvent, Error>> + Send>>,
    /// Cancellation hook the adapter fires when the response is dropped.
    /// Implementors translate this into the kernel's `stream.cancel`
    /// notification (for process / wasm) or into firing the per-call
    /// `Cancel` handle (for builtin).
    pub cancel: CancelHandle,
}

/// Opaque cancel hook. Fires its inner function exactly once, on
/// `Drop` or on explicit [`Self::fire`]. Held by the SSE / NDJSON
/// response wrappers so the cancellation path is "the response body
/// dropped" → "the extension sees cancel".
pub struct CancelHandle {
    fired: Arc<AtomicBool>,
    // `Mutex` makes the handle `Sync` so the response Drop guard can
    // sit inside `http::Extensions` (which requires `Send + Sync`)
    // without an extra wrapper. The mutex is uncontended — `fire`
    // takes the lock for the duration of the user-supplied closure
    // exactly once.
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
}

impl Drop for CancelHandle {
    fn drop(&mut self) {
        self.fire();
    }
}

impl std::fmt::Debug for CancelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancelHandle")
            .field("fired", &self.fired.load(Ordering::SeqCst))
            .finish()
    }
}

/// The adapter↔kernel seam. One method per call shape so the adapter
/// reads the manifest's `streaming:` mode at build time and never has
/// to inspect bodies at request time to decide which path to take.
///
/// `caller` carries the principal the inbound HTTP request was
/// dispatched on behalf of. Hosts populate it by stuffing a
/// [`CallerIdentity`] into the axum request extensions (typically
/// from an auth middleware); the REST handler reads it back out and
/// hands it to the dispatcher. `None` means the host did not
/// authenticate / scope the request — capability backends that
/// require a tenant refuse such frames with
/// [`starter_ext_spi::Error::Capability`].
#[async_trait]
pub trait RestDispatcher: Send + Sync + 'static {
    /// Dispatch a non-streaming call. The dispatcher owns the lifetime
    /// of the call; the adapter renders the returned value as the
    /// response body.
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError>;

    /// Open a streaming dispatch. Same contract as [`Self::dispatch`]
    /// but returns a [`StreamResponse`] the adapter pumps onto the
    /// wire.
    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<StreamResponse, DispatchError>;
}

// ---------------------------------------------------------------------------
// BuiltinRestDispatcher
// ---------------------------------------------------------------------------

/// Routes calls to a `BuiltinTable`. The host that statically links
/// extensions hands its populated `BuiltinTable` to the adapter, plus
/// the `ExtensionRegistry` for runtime-kind lookups.
pub struct BuiltinRestDispatcher {
    table: Arc<BuiltinTable>,
    registry: Arc<ExtensionRegistry>,
    /// Per-stream channel capacity. 16 is large enough that a fast
    /// handler doesn't immediately backpressure on a slow client; the
    /// adapter's renderer drains at SSE / NDJSON wire speed.
    event_channel_capacity: usize,
    /// Source of capability backends per call. Defaults to
    /// [`StubCapabilityFactory`] (every backend refuses with
    /// `Error::Capability`). Hosts opt in to real backends via
    /// [`Self::with_capability_factory`].
    capability_factory: Arc<dyn CapabilityFactory>,
    /// Opt-in cache wiring. `None` means cache is disabled — every
    /// dispatch goes straight through to the handler. v0 integration
    /// point for the opt-in caching proposal.
    cache: Option<DispatcherCache>,
}

impl BuiltinRestDispatcher {
    /// New builtin dispatcher.
    pub fn new(table: Arc<BuiltinTable>, registry: Arc<ExtensionRegistry>) -> Self {
        Self {
            table,
            registry,
            event_channel_capacity: 16,
            capability_factory: Arc::new(StubCapabilityFactory),
            cache: None,
        }
    }

    /// Install a [`DispatcherCache`] for opt-in kind caching. The
    /// dispatcher consults the registry on every `dispatch()`; kinds
    /// without a sidecar take the no-op path. Streaming dispatch is
    /// never cached — actions and streams are out of scope for v0.
    pub fn with_cache(mut self, cache: DispatcherCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Install a host-provided [`CapabilityFactory`]. Builder-style
    /// so existing callers can keep their single-line construction
    /// unchanged.
    ///
    /// The factory is consulted every time the dispatcher builds a
    /// `Ctx` (once per dispatch). It's the host's job to make
    /// per-call construction cheap (the rubix-side factory clones
    /// `Arc`s only).
    pub fn with_capability_factory(mut self, factory: Arc<dyn CapabilityFactory>) -> Self {
        self.capability_factory = factory;
        self
    }

    /// Construct a per-call `CtxInner`. The host-overridable
    /// capabilities (`warehouse_read`, `event_bus`) come from
    /// [`Self::capability_factory`]; everything else is the
    /// substrate-default stub for now (lifting those to the factory
    /// is a follow-up — `secrets`/`http_out`/`fs` each want their
    /// own allowlist threading first).
    ///
    /// `caller` is the principal the inbound frame was dispatched
    /// on behalf of (`None` for system / host-internal frames).
    /// `extension` is the id of the extension whose `Ctx` is being
    /// built — the factory uses it as the publish-side namespace
    /// for the event bus and as the lookup key for per-extension
    /// manifest grants.
    fn build_ctx(
        &self,
        extension: &ExtensionId,
        events: mpsc::Sender<StreamEvent>,
        cancel: Arc<dyn Cancel>,
        caller: Option<&starter_ext_spi::identity::CallerIdentity>,
    ) -> CtxInner {
        CtxInner::new(
            events,
            cancel,
            Arc::new(StubSecrets),
            Arc::new(StubHttpOut),
            Arc::new(StubFs),
            Arc::new(StubWallClock),
            Arc::new(StubTracing),
            self.capability_factory.warehouse_read(extension, caller),
            self.capability_factory.warehouse_write(extension, caller),
            self.capability_factory.event_bus(extension, caller),
            self.capability_factory.dashboard(extension, caller),
            self.capability_factory.authz(extension, caller),
        )
    }

    fn ensure_builtin(&self, extension: &ExtensionId) -> Result<(), DispatchError> {
        let rec = self.registry.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!("extension {:?}", extension.as_str()))
        })?;
        let manifest = rec
            .manifest
            .as_ref()
            .ok_or_else(|| DispatchError::NotFound("manifest missing".into()))?;
        match manifest.runtime.kind {
            RuntimeKind::Builtin => Ok(()),
            other => Err(DispatchError::NotWired(format!(
                "REST dispatch for runtime kind {other:?} ships in the next adapter slice; \
                 builtin only in this phase"
            ))),
        }
    }
}

impl BuiltinRestDispatcher {
    /// Inner dispatch — the original (uncached) path. Factored out
    /// so the public `dispatch` can wrap it in the cache layer when
    /// a spec is registered for `(extension, contribute_id)`.
    async fn dispatch_inner(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<&CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        self.ensure_builtin(extension)?;
        let entry = self.table.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!(
                "extension {:?} declares builtin runtime but is absent from the BuiltinTable — \
                 was `register_static_table!` called?",
                extension.as_str()
            ))
        })?;

        // Non-streaming: stub event channel + cancel (handler may
        // ignore them).
        let (tx, _rx) = mpsc::channel(self.event_channel_capacity);
        let ctx = self.build_ctx(extension, tx, Arc::new(NeverCancel), caller);

        let entry_dispatch = entry.dispatch_arc();
        let contribute_id_owned = contribute_id.to_owned();
        let result: starter_ext_spi::Result<serde_json::Value> =
            tokio::task::spawn_blocking(move || entry_dispatch(&contribute_id_owned, &ctx, input))
                .await
                .map_err(|e| DispatchError::Substrate(format!("dispatch join: {e}")))?;
        result.map_err(DispatchError::from_kernel)
    }
}

#[async_trait]
impl RestDispatcher for BuiltinRestDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        // Fast path: no cache configured, or no spec for this kind.
        let spec = self
            .cache
            .as_ref()
            .and_then(|c| c.registry.get(extension, contribute_id).cloned());

        let Some(spec) = spec else {
            return self
                .dispatch_inner(extension, contribute_id, input, caller.as_ref())
                .await;
        };

        // Cached path: wrap the inner dispatch in the cache layer.
        // Loader serialises the JSON result to bytes; cache hits
        // round-trip back through serde.
        let cache = self
            .cache
            .as_ref()
            .expect("cache must be Some — spec lookup succeeded");
        let layer = cache.layer.clone();
        let scope = caller_scope(caller.as_ref());
        let base_key = dispatch_base_key(extension, contribute_id, &input);

        let extension_owned = extension.clone();
        let contribute_id_owned = contribute_id.to_owned();
        let caller_for_load = caller.clone();
        let this = self;
        let bytes = layer
            .get_or_load(&spec, &scope, &base_key, move || async move {
                let v = this
                    .dispatch_inner(
                        &extension_owned,
                        &contribute_id_owned,
                        input,
                        caller_for_load.as_ref(),
                    )
                    .await?;
                let serialised = serde_json::to_vec(&v).map_err(|e| {
                    DispatchError::Substrate(format!("cache: serialise dispatch result: {e}"))
                })?;
                Ok::<_, DispatchError>(std::sync::Arc::new(serialised))
            })
            .await?;

        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            DispatchError::Substrate(format!("cache: deserialise stored result: {e}"))
        })?;
        Ok(value)
    }

    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<StreamResponse, DispatchError> {
        self.ensure_builtin(extension)?;
        let entry = self.table.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!(
                "extension {:?} absent from the BuiltinTable",
                extension.as_str()
            ))
        })?;

        // Allocate a stream id and a cancel watch the handler observes.
        let stream_id = StreamId(format!("rest-{}-{}", extension.as_str(), uuid_like_short(),));
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let cancel = WatchCancel::new(cancel_rx);
        let (tx, rx) = mpsc::channel(self.event_channel_capacity);

        let ctx = self.build_ctx(extension, tx, Arc::new(cancel), caller.as_ref());
        let entry_dispatch = entry.dispatch_arc();
        let contribute_id_owned = contribute_id.to_owned();

        // Fire the handler on a blocking task. When the handler
        // returns, the `EventSender` is dropped, which closes `rx` and
        // ends the stream cleanly (an implicit `stream.end`).
        tokio::task::spawn_blocking(move || {
            let _ = entry_dispatch(&contribute_id_owned, &ctx, input);
        });

        // The stream yields whatever the handler emits. We map
        // mpsc::Receiver into a Stream and tag each item with `Ok`;
        // the dispatcher exposes a `Result` shape so process / wasm
        // dispatchers can yield in-band `stream.error` payloads later.
        let events = Box::pin(ReceiverStream::new(rx).map(Ok));

        // Cancel handle: when fired, signals every watcher inside the
        // builtin handler that cancellation is requested. Build inside
        // a spawn so the close-channel handshake is non-blocking.
        let cancel_handle = CancelHandle::new(move || {
            // We must outlive `dispatch_stream` (the cancel fires from
            // the response Drop). Sending into a watch is sync.
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
// NotWiredDispatcher
// ---------------------------------------------------------------------------

/// Default dispatcher for hosts that haven't wired anything yet. Every
/// call returns `DispatchError::NotWired`. Routers built against this
/// dispatcher answer 503 for every contributed entry; the admin slice
/// stays fully functional.
#[derive(Debug, Default)]
pub struct NotWiredDispatcher;

#[async_trait]
impl RestDispatcher for NotWiredDispatcher {
    async fn dispatch(
        &self,
        _extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no RestDispatcher wired for contribute {contribute_id:?}"
        )))
    }

    async fn dispatch_stream(
        &self,
        _extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _caller: Option<CallerIdentity>,
    ) -> Result<StreamResponse, DispatchError> {
        Err(DispatchError::NotWired(format!(
            "no RestDispatcher wired for streaming contribute {contribute_id:?}"
        )))
    }
}

// ---------------------------------------------------------------------------
// ProcessRestDispatcher — process-flavour synchronous JSON-RPC dispatch
// ---------------------------------------------------------------------------

/// Process-flavour REST dispatcher.
///
/// Holds the per-extension [`starter_ext_supervisor::SupervisorHandle`]
/// map and a default per-call timeout. Synchronous unary dispatch is
/// implemented on top of [`starter_ext_supervisor::SupervisorHandle::call`]:
/// the request method on the wire is `tools/<contribute_id>` so the
/// `starter-ext-sdk` `register_process_main!`-generated child loop
/// routes the call through `ExtensionDispatch::dispatch_tool`. Streaming
/// dispatch remains `NotWired` until the per-stream demultiplexer for
/// `stream.event` / `stream.end` notifications lands additively.
pub struct ProcessRestDispatcher {
    handles: std::collections::HashMap<ExtensionId, Arc<starter_ext_supervisor::SupervisorHandle>>,
    request_timeout: Duration,
    /// Opt-in cache wiring. Same shape as
    /// [`BuiltinRestDispatcher::with_cache`]. v0 enables this for the
    /// rubixos `usage_bucketed` canary; all other process kinds
    /// without a sidecar take the no-op path.
    cache: Option<DispatcherCache>,
}

impl ProcessRestDispatcher {
    /// New process dispatcher with a default per-call timeout.
    pub fn new(
        handles: std::collections::HashMap<
            ExtensionId,
            Arc<starter_ext_supervisor::SupervisorHandle>,
        >,
        request_timeout: Duration,
    ) -> Self {
        Self {
            handles,
            request_timeout,
            cache: None,
        }
    }

    /// Install a [`DispatcherCache`] for opt-in process-kind caching.
    /// Streaming dispatch is never cached.
    pub fn with_cache(mut self, cache: DispatcherCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Default per-call timeout the consumer configured.
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

    /// The uncached dispatch path — preserves the existing behaviour
    /// so the cache wrapper is purely additive.
    async fn dispatch_inner(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        let handle = self.handles.get(extension).ok_or_else(|| {
            DispatchError::NotFound(format!(
                "no supervisor handle for extension {:?}",
                extension.as_str()
            ))
        })?;
        let method = format!("tools/{contribute_id}");
        let ext_str = extension.as_str().to_string();
        let timeout = self.request_timeout;
        debug!(
            ext = %ext_str,
            method = %method,
            timeout_ms = timeout.as_millis() as u64,
            "REST tool dispatch begin",
        );
        let started = std::time::Instant::now();
        let result = match caller {
            Some(caller) => handle
                .call_as(&method, input, caller, timeout)
                .await
                .map_err(DispatchError::from_kernel),
            None => handle
                .call(&method, input, timeout)
                .await
                .map_err(DispatchError::from_kernel),
        };
        let elapsed_ms = started.elapsed().as_millis() as u64;
        match &result {
            Ok(_) => debug!(
                ext = %ext_str,
                method = %method,
                elapsed_ms,
                "REST tool dispatch ok",
            ),
            Err(e) => warn!(
                ext = %ext_str,
                method = %method,
                elapsed_ms,
                error = %e,
                "REST tool dispatch failed",
            ),
        }
        result
    }
}

#[async_trait]
impl RestDispatcher for ProcessRestDispatcher {
    async fn dispatch(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        input: serde_json::Value,
        caller: Option<CallerIdentity>,
    ) -> Result<serde_json::Value, DispatchError> {
        // Fast path: no cache configured, or no spec for this kind.
        let spec = self
            .cache
            .as_ref()
            .and_then(|c| c.registry.get(extension, contribute_id).cloned());
        let Some(spec) = spec else {
            return self
                .dispatch_inner(extension, contribute_id, input, caller)
                .await;
        };

        let cache = self.cache.as_ref().expect("cache must be Some");
        let layer = cache.layer.clone();
        let scope = caller_scope(caller.as_ref());
        let base_key = dispatch_base_key(extension, contribute_id, &input);

        let extension_owned = extension.clone();
        let contribute_id_owned = contribute_id.to_owned();
        let caller_for_load = caller.clone();
        let this = self;
        let bytes = layer
            .get_or_load(&spec, &scope, &base_key, move || async move {
                let v = this
                    .dispatch_inner(
                        &extension_owned,
                        &contribute_id_owned,
                        input,
                        caller_for_load,
                    )
                    .await?;
                let serialised = serde_json::to_vec(&v).map_err(|e| {
                    DispatchError::Substrate(format!("cache: serialise dispatch result: {e}"))
                })?;
                Ok::<_, DispatchError>(std::sync::Arc::new(serialised))
            })
            .await?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            DispatchError::Substrate(format!("cache: deserialise stored result: {e}"))
        })?;
        Ok(value)
    }

    async fn dispatch_stream(
        &self,
        extension: &ExtensionId,
        contribute_id: &str,
        _input: serde_json::Value,
        _caller: Option<CallerIdentity>,
    ) -> Result<StreamResponse, DispatchError> {
        if self.handles.contains_key(extension) {
            Err(DispatchError::NotWired(format!(
                "process-flavour streaming REST dispatch for {contribute_id:?} \
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

// ---------------------------------------------------------------------------
// Internals: stub capability backends, NeverCancel-style cancel-on-disconnect
//
// `warehouse_read` and `event_bus` are no longer stubbed here — the
// `CapabilityFactory` seam in `crate::capabilities` owns those.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct StubSecrets;
impl SecretsBackend for StubSecrets {
    fn get(&self, _name: &str) -> starter_ext_spi::Result<String> {
        Err(Error::capability(
            "secrets not wired in REST adapter Phase 5",
        ))
    }
}

#[derive(Debug)]
struct StubHttpOut;
impl HttpOutBackend for StubHttpOut {
    fn request(&self, _req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        Err(Error::capability(
            "http_out not wired in REST adapter Phase 5",
        ))
    }
}

#[derive(Debug)]
struct StubFs;
impl FsBackend for StubFs {
    fn read(&self, _path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        Err(Error::capability("fs not wired in REST adapter Phase 5"))
    }
}

#[derive(Debug)]
struct StubWallClock;
impl WallClockBackend for StubWallClock {
    fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        Err(Error::capability(
            "wall_clock not wired in REST adapter Phase 5",
        ))
    }
}

#[derive(Debug)]
struct StubTracing;
impl TracingBackend for StubTracing {
    fn event(&self, _level: &str, _msg: &str, _fields: serde_json::Value) {}
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

/// `Cancel` impl backed by a `watch::Receiver<bool>`. Set to `true` from
/// the response's drop guard; the extension observes it via
/// `ctx.cancel().is_cancelled()` or `.cancelled().await`.
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
            // Already cancelled? Resolve immediately.
            if *rx.borrow() {
                return;
            }
            // Poll until the watch turns true. `changed()` returns
            // `Err` only if every sender dropped — treat that as
            // "session ended" and return so the handler can wind down.
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

// ---------------------------------------------------------------------------
// Tiny utilities — no dep on `uuid`, no dep on `tokio-stream`.
// ---------------------------------------------------------------------------

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

/// `tokio_stream::wrappers::ReceiverStream` minus the dep. Reads items
/// from a `mpsc::Receiver` and yields them as a `futures::Stream`.
struct ReceiverStream<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> ReceiverStream<T> {
    fn new(rx: mpsc::Receiver<T>) -> Self {
        Self { rx }
    }
}

impl<T: Send + 'static> Stream for ReceiverStream<T> {
    type Item = T;
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        self.rx.poll_recv(cx)
    }
}

use futures::StreamExt;

// ---------------------------------------------------------------------------
// Helpers consumed by handler.rs
// ---------------------------------------------------------------------------

impl StreamResponse {
    /// Render one outbound payload as a `StreamNotification::Event` so
    /// the adapter's renderer can emit it as either an SSE `data:`
    /// field or an NDJSON line with one uniform JSON shape. Surfaced
    /// publicly for adapter tests that round-trip the wire shape.
    pub fn to_notification(stream_id: &StreamId, payload: serde_json::Value) -> StreamNotification {
        StreamNotification::Event(starter_ext_spi::jsonrpc::StreamEvent {
            stream_id: stream_id.clone(),
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancel_handle_runs_on_drop_exactly_once() {
        let counter = Arc::new(AtomicBool::new(false));
        let c = counter.clone();
        let handle = CancelHandle::new(move || {
            c.store(true, Ordering::SeqCst);
        });
        drop(handle);
        assert!(counter.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancel_handle_fire_then_drop_is_idempotent() {
        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let handle = CancelHandle::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        handle.fire();
        drop(handle);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
