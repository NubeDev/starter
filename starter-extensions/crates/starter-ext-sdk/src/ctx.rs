//! The internal `Ctx` machinery that `requires!{}` builds a per-extension
//! newtype on top of.
//!
//! Per SCOPE.md **R6**: the `Ctx` an extension receives only exposes
//! methods for the capability categories it declared in `requires!{}`. No
//! untyped `host_call(method, params)` escape hatch in v0.1 — adding a new
//! host method is an additive trait extension here (and an additive
//! `Capability` variant in `starter-ext-spi`), not a string key.
//!
//! Per Stage 4 of the implementation plan: every `Ctx` carries a
//! `Stream<Item = Event>` shape mirroring `starter-spi::ai::OnEvent +
//! Cancel`. Both ends of that pair are kernel-level (post-R13 stream
//! sub-protocol) and live on the shared inner handle, not behind a
//! capability gate — every extension can emit events and observe
//! cancellation.
//!
//! The handles in this module are *opaque wrappers*: their method sets are
//! intentionally small in Phase 1 (Stage 4 lands the typed surface; the
//! adapter crates and the host implementation flesh out the bodies in
//! later stages). Returning `&Handle` from `Ctx` is what keeps the
//! `requires!`-generated newtype `Clone` and zero-cost.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use starter_ext_spi::jsonrpc::StreamId;

// ---------------------------------------------------------------------------
// Event stream (mirror of `starter-spi::ai::OnEvent + Cancel`)
// ---------------------------------------------------------------------------

/// One streaming-event payload an extension can emit back to the host.
///
/// Shape lifted from `starter-spi::ai::Event` but kept independent — the
/// extension substrate's event vocabulary is not "AI tokens", it is
/// transport-agnostic `payload` chunks that adapters translate into their
/// transport's native frame (SSE, gRPC server-streaming, MCP
/// notifications). The host wraps each emitted `Event` in a
/// `stream.event` JSON-RPC notification carrying the originating
/// `stream_id` (SCOPE post-R13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// The stream this chunk belongs to. Allocated by the host on the
    /// initiating call and threaded through every chunk.
    pub stream_id: StreamId,
    /// Opaque payload. Adapter-specific shape; the kernel does not
    /// interpret it.
    pub payload: serde_json::Value,
}

/// Bounded `Sender<Event>` used to emit chunks back to the host. Mirror of
/// `starter-spi::ai::OnEvent`.
///
/// **Backpressure semantics.** The channel is bounded; capacity is the
/// host's choice when constructing the underlying `mpsc::channel`. Sync
/// callers `try_send` and drop on overflow (a `tracing::warn!` records the
/// drop); async callers `.await` each send and let backpressure flow.
pub type EventSender = mpsc::Sender<Event>;

/// Matching `Receiver<Event>`. The host owns one of these per in-flight
/// streaming call and drives it onto the wire.
pub type EventReceiver = mpsc::Receiver<Event>;

/// Compatibility alias mirroring `starter-spi::ai::OnEvent`. Kept so code
/// migrating from the AI runner pattern reads the same.
pub type EmitEvent = EventSender;

/// Cancellation handle the extension polls or selects against.
///
/// Mirror of `starter-spi::ai::Cancel`: both shapes are provided so
/// handlers can integrate with whichever pattern fits their loop — sync
/// handlers poll `is_cancelled()` between long steps, async handlers
/// `.select!` against `cancelled().await`.
pub trait Cancel: Send + Sync + 'static {
    /// `true` once the host has requested cancellation (operator
    /// `disable`, supervisor shutdown, `stream.cancel` notification).
    fn is_cancelled(&self) -> bool;

    /// A future that resolves when cancellation is requested. Lifetime is
    /// bound to `&self` so the implementor can hold internal state
    /// without an `Arc`.
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

/// A no-op `Cancel` impl used by `MockCtx` and by handlers that do not
/// need cancellation. Always reports "not cancelled" and parks its
/// future forever.
///
/// Kept here (rather than behind a `testing` feature) so the
/// `requires!{}` macro can name it from generated code without forcing
/// `cfg(test)` on the extension.
#[derive(Debug, Default, Clone, Copy)]
pub struct NeverCancel;

impl Cancel for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }

    fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

// ---------------------------------------------------------------------------
// Per-capability handles
//
// Each handle is intentionally opaque in Stage 4. The method bodies the
// SDK exposes here are the *shape* the host wires up in `starter-ext-host`
// (builtin), `starter-ext-supervisor` (process), and `starter-ext-wasm`
// (wasm). The Stage 4 SDK ships the types so `requires!{}` can name them;
// the runtime behaviour lands with the host crate.
// ---------------------------------------------------------------------------

/// Secret-store access (granted by `capabilities.secrets:`).
#[derive(Debug, Clone)]
pub struct SecretsHandle {
    inner: Arc<dyn private::SecretsBackend>,
}

impl SecretsHandle {
    /// Fetch a secret by name. The host enforces the prefix allowlist
    /// declared in `block.yaml`; a fetch outside the allowlist returns
    /// `Error::Capability`.
    pub fn get(&self, name: &str) -> starter_ext_spi::Result<String> {
        self.inner.get(name)
    }
}

/// Outbound-HTTP handle (granted by `capabilities.http_out:`).
#[derive(Debug, Clone)]
pub struct HttpOutHandle {
    inner: Arc<dyn private::HttpOutBackend>,
}

impl HttpOutHandle {
    /// Issue an outbound HTTP request. The host enforces the authority
    /// allowlist declared in `block.yaml`.
    pub fn request(&self, req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        self.inner.request(req)
    }
}

/// Filesystem-access handle (granted by `capabilities.fs:`).
#[derive(Debug, Clone)]
pub struct FsHandle {
    inner: Arc<dyn private::FsBackend>,
}

impl FsHandle {
    /// Read a file relative to a granted path spec.
    pub fn read(&self, path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        self.inner.read(path)
    }
}

/// Wall-clock handle (granted by `capabilities.wall_clock: true`).
#[derive(Debug, Clone)]
pub struct WallClockHandle {
    inner: Arc<dyn private::WallClockBackend>,
}

impl WallClockHandle {
    /// Current Unix epoch time in milliseconds.
    pub fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        self.inner.now_unix_ms()
    }
}

/// Structured-tracing handle (always granted; declared as a capability so
/// the manifest documents which extensions emit telemetry).
#[derive(Debug, Clone)]
pub struct TracingHandle {
    inner: Arc<dyn private::TracingBackend>,
}

impl TracingHandle {
    /// Emit a structured event at the given level.
    pub fn event(&self, level: &str, msg: &str, fields: serde_json::Value) {
        self.inner.event(level, msg, fields)
    }
}

// ---------------------------------------------------------------------------
// CtxInner — the always-shared backing handle.
//
// The `requires!{}` newtype wraps one of these. Methods on the newtype
// return `&Handle` views into this struct; capability accessors return
// the per-capability `*Handle` types above. The host constructs `CtxInner`
// once per extension and clones cheaply (everything is `Arc`-backed).
// ---------------------------------------------------------------------------

/// SDK-internal Ctx backing. Never constructed by extension code — the
/// per-flavour entry-point glue (builtin, process, wasm) builds one and
/// hands it to the `requires!{}`-generated newtype via
/// `__from_inner`.
#[derive(Clone)]
pub struct CtxInner {
    events: EventSender,
    cancel: Arc<dyn Cancel>,
    secrets: SecretsHandle,
    http_out: HttpOutHandle,
    fs: FsHandle,
    wall_clock: WallClockHandle,
    tracing: TracingHandle,
}

impl CtxInner {
    /// Construct a fully-wired `CtxInner` from per-category backends.
    ///
    /// Called by the per-flavour entry-point glue; not part of the
    /// extension-author surface.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        events: EventSender,
        cancel: Arc<dyn Cancel>,
        secrets: Arc<dyn private::SecretsBackend>,
        http_out: Arc<dyn private::HttpOutBackend>,
        fs: Arc<dyn private::FsBackend>,
        wall_clock: Arc<dyn private::WallClockBackend>,
        tracing: Arc<dyn private::TracingBackend>,
    ) -> Self {
        Self {
            events,
            cancel,
            secrets: SecretsHandle { inner: secrets },
            http_out: HttpOutHandle { inner: http_out },
            fs: FsHandle { inner: fs },
            wall_clock: WallClockHandle { inner: wall_clock },
            tracing: TracingHandle { inner: tracing },
        }
    }

    /// Returns the always-present event sender.
    pub fn events(&self) -> &EventSender {
        &self.events
    }

    /// Returns the always-present cancellation handle.
    pub fn cancel(&self) -> &dyn Cancel {
        &*self.cancel
    }

    /// Borrow the secrets handle (named by `requires!(secrets)`).
    pub fn secrets(&self) -> &SecretsHandle {
        &self.secrets
    }
    /// Borrow the outbound-HTTP handle (named by `requires!(http_out)`).
    pub fn http_out(&self) -> &HttpOutHandle {
        &self.http_out
    }
    /// Borrow the filesystem handle (named by `requires!(fs)`).
    pub fn fs(&self) -> &FsHandle {
        &self.fs
    }
    /// Borrow the wall-clock handle (named by `requires!(wall_clock)`).
    pub fn wall_clock(&self) -> &WallClockHandle {
        &self.wall_clock
    }
    /// Borrow the tracing handle (named by `requires!(tracing)`).
    pub fn tracing(&self) -> &TracingHandle {
        &self.tracing
    }
}

impl std::fmt::Debug for CtxInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CtxInner").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Backend traits.
//
// Kept in a `pub(crate)`-visible sub-module so the public capability
// handles can name them without leaking implementor concerns into the
// SDK's public namespace. The host crate implements each trait against
// its concrete backing (secret store, reqwest client, std::fs, …).
// ---------------------------------------------------------------------------

pub use private::{FsBackend, HttpOutBackend, SecretsBackend, TracingBackend, WallClockBackend};

mod private {
    /// Host-side backing for [`super::SecretsHandle`].
    pub trait SecretsBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Fetch a secret by name.
        fn get(&self, name: &str) -> starter_ext_spi::Result<String>;
    }

    /// Host-side backing for [`super::HttpOutHandle`].
    pub trait HttpOutBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Issue an outbound HTTP request encoded as JSON.
        fn request(
            &self,
            req: serde_json::Value,
        ) -> starter_ext_spi::Result<serde_json::Value>;
    }

    /// Host-side backing for [`super::FsHandle`].
    pub trait FsBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Read a file relative to a granted path spec.
        fn read(&self, path: &str) -> starter_ext_spi::Result<Vec<u8>>;
    }

    /// Host-side backing for [`super::WallClockHandle`].
    pub trait WallClockBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Current Unix epoch time in milliseconds.
        fn now_unix_ms(&self) -> starter_ext_spi::Result<u64>;
    }

    /// Host-side backing for [`super::TracingHandle`].
    pub trait TracingBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Emit a structured event at the given level.
        fn event(&self, level: &str, msg: &str, fields: serde_json::Value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::jsonrpc::StreamId;

    #[test]
    fn event_round_trip() {
        let e = Event {
            stream_id: StreamId("s-1".into()),
            payload: serde_json::json!({ "line": "hello" }),
        };
        let j = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&j).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn never_cancel_is_never_cancelled() {
        let c = NeverCancel;
        assert!(!c.is_cancelled());
        // We do not poll the future — `pending()` would block forever.
    }
}
