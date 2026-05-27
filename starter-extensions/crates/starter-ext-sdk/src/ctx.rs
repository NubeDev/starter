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

pub use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::jsonrpc::StreamId;
pub use starter_ext_spi::warehouse::{Row, TemplateSpec, WarehouseReadRequest};
pub use starter_ext_spi::event_bus::{EventBusMessage, EventBusPublishRequest};

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

/// Named-template warehouse-read handle (granted by
/// `capabilities.warehouse_read.tables: […]`).
///
/// The handle is **not** a SQL gateway. Templates are server-defined;
/// extensions reference them by name and bind typed parameters. The
/// host's registry rejects unknown templates and the supervisor's
/// capability gate refuses templates touching tables outside the
/// extension's grant.
///
/// Per Appendix A of the extension-architecture-north-star proposal,
/// the host binds `$caller_tenant_id` from `ctx.caller()` before
/// executing the template — extensions cannot override it. A frame
/// without a caller (system frame) is refused by the host-side
/// backend with `Error::Capability`.
#[derive(Debug, Clone)]
pub struct WarehouseReadHandle {
    inner: Arc<dyn private::WarehouseReadBackend>,
}

impl WarehouseReadHandle {
    /// Execute a named-template query, returning the rows.
    ///
    /// `template` must exist in the host's registry (a `block.yaml`
    /// contribution under `contributes.warehouse_templates[]` once row
    /// 3 lands, or one of the host's builtin templates). `params` is
    /// validated against the template's parameter schema before
    /// binding.
    ///
    /// v1 returns a `Vec<Row>` (sync). A future v2 will add
    /// `query_stream(…) -> BoxStream<Row>` for large pulls without
    /// pinning a host worker on the buffer; the v1 shape is preserved
    /// per the proposal's "add v2 method, keep v1" deprecation rule.
    pub fn query(
        &self,
        template: &str,
        params: serde_json::Value,
    ) -> starter_ext_spi::Result<Vec<Row>> {
        self.inner.query(template, params)
    }

    /// Count of rows the template would yield. Cheaper than `query`
    /// when the caller only needs a tally (KPI cards, pagination).
    pub fn count(
        &self,
        template: &str,
        params: serde_json::Value,
    ) -> starter_ext_spi::Result<u64> {
        self.inner.count(template, params)
    }

    /// Look up a template's catalog entry (parameter schema, allowed
    /// tables, optional SQL body). Returns `None` if the template is
    /// not registered. Useful for documentation surfaces and for
    /// extensions that want to introspect their own grant before
    /// invoking.
    pub fn describe(&self, template: &str) -> starter_ext_spi::Result<Option<TemplateSpec>> {
        self.inner.describe(template)
    }
}

/// Warehouse-write handle (granted by
/// `capabilities.warehouse_write.tables: […]`).
///
/// Companion to [`WarehouseReadHandle`]. Lets an extension insert
/// rows into tables it declared in `contributes.warehouse_tables[]`.
/// The host:
///
/// - Refuses system-frame calls (no caller tenant) with
///   `Error::Capability`.
/// - Gates the call against the extension's grant — `table` must be
///   in `capabilities.warehouse_write.tables`.
/// - Stamps every row's `tenant_id` to the caller's tenant before
///   issuing the INSERT, overwriting any value the extension tried
///   to set. Extensions cannot spoof cross-tenant writes.
/// - Validates the row's columns against the table schema declared
///   in the manifest; unknown or mistyped columns refuse with
///   `Error::Validation`.
///
/// `table` is the unprefixed name as declared in the manifest. The
/// host resolves it to `<sanitized_extension_id>__<table>` before
/// issuing DDL.
#[derive(Debug, Clone)]
pub struct WarehouseWriteHandle {
    inner: Arc<dyn private::WarehouseWriteBackend>,
}

impl WarehouseWriteHandle {
    /// Insert `rows` into `table`. Returns the number of rows the
    /// engine acknowledged.
    ///
    /// Each row is a column-name → JSON value map. The host strips
    /// any `tenant_id` field the extension supplied and replaces it
    /// with the caller's tenant. Missing columns are filled from the
    /// manifest's `default` expression when one is declared; otherwise
    /// the call refuses with `Error::Validation`.
    pub fn insert(&self, table: &str, rows: Vec<Row>) -> starter_ext_spi::Result<u64> {
        self.inner.insert(table, rows)
    }
}

/// In-process publish/subscribe bus handle (granted by
/// `capabilities.event_bus.publish:` / `subscribe:`).
///
/// Per [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
/// row 4. Cross-filter and live updates push through the bus
/// instead of polling via N HTTP loopback round-trips.
///
/// V1 exposes `publish` only. The subscribe-side handle
/// (`subscribe(topic) -> BoxStream<EventBusMessage>`) is a
/// follow-up — the SPI wire request type for it
/// (`EventBusSubscribeRequest`) ships in the same release so the
/// shape doesn't break when the handle lands.
#[derive(Debug, Clone)]
pub struct EventBusHandle {
    inner: Arc<dyn private::EventBusBackend>,
}

impl EventBusHandle {
    /// Publish a message on `topic`.
    ///
    /// `topic` must be in the extension's grant `publish: [...]`
    /// allowlist and must live under the extension's reverse-DNS
    /// namespace; the supervisor refuses cross-namespace publishes.
    /// The host stamps a `ts_unix_ms` value before fan-out so every
    /// subscriber sees the same timestamp.
    pub fn publish(
        &self,
        topic: &str,
        payload: serde_json::Value,
    ) -> starter_ext_spi::Result<()> {
        self.inner.publish(topic, payload)
    }
}

/// SDUI dashboard read/write handle (granted by
/// `capabilities.dashboard.{read,write}:` allowlists).
///
/// Row 5 of the extension-north-star: extensions that contribute
/// SDUI surfaces reach the host's dashboard store through this
/// handle instead of issuing loopback HTTP. The host binds the
/// caller's `tenant_id` from `ctx.caller()` before resolving any
/// page id — extensions cannot read or write rows from another
/// tenant. A system frame (no caller) is refused with
/// `Error::Capability`.
#[derive(Debug, Clone)]
pub struct DashboardHandle {
    inner: Arc<dyn private::DashboardBackend>,
}

impl DashboardHandle {
    /// Read the page body for `page_id` under the caller's tenant.
    ///
    /// Page bodies are opaque JSON to the SDK — the SDUI layer
    /// keeps the schema. Returns the raw body the host stored;
    /// `Error::NotFound` when the (tenant, page_id) pair does
    /// not resolve.
    pub fn read(&self, page_id: &str) -> starter_ext_spi::Result<serde_json::Value> {
        self.inner.read(page_id)
    }

    /// Replace the page body for `page_id` under the caller's
    /// tenant.
    ///
    /// Idempotent overwrite; the host appends a changelog entry
    /// attributing the write to `ctx.caller().user_id`.
    pub fn write(
        &self,
        page_id: &str,
        body: serde_json::Value,
    ) -> starter_ext_spi::Result<()> {
        self.inner.write(page_id, body)
    }
}

/// Authorization-check handle (granted by
/// `capabilities.authz.check:` allowlists).
///
/// Row 5 of the extension-north-star: extensions that compose
/// new flows on top of the host's authz engine ask "is the
/// caller allowed to do X on Y?" through this handle, so the
/// engine (and its rule set) stays the single source of truth.
/// The host binds the caller's `Principal` from
/// `ctx.caller()` before evaluating the rule; a system frame
/// (no caller) is refused with `Error::Capability`.
#[derive(Debug, Clone)]
pub struct AuthzHandle {
    inner: Arc<dyn private::AuthzBackend>,
}

impl AuthzHandle {
    /// Evaluate `action` against `resource` for the calling
    /// principal. `true` if the action is permitted.
    pub fn check(&self, action: &str, resource: &str) -> starter_ext_spi::Result<bool> {
        self.inner.check(action, resource)
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
    caller: Option<Arc<CallerIdentity>>,
    secrets: SecretsHandle,
    http_out: HttpOutHandle,
    fs: FsHandle,
    wall_clock: WallClockHandle,
    tracing: TracingHandle,
    warehouse_read: WarehouseReadHandle,
    warehouse_write: WarehouseWriteHandle,
    event_bus: EventBusHandle,
    dashboard: DashboardHandle,
    authz: AuthzHandle,
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
        warehouse_read: Arc<dyn private::WarehouseReadBackend>,
        warehouse_write: Arc<dyn private::WarehouseWriteBackend>,
        event_bus: Arc<dyn private::EventBusBackend>,
        dashboard: Arc<dyn private::DashboardBackend>,
        authz: Arc<dyn private::AuthzBackend>,
    ) -> Self {
        Self {
            events,
            cancel,
            caller: None,
            secrets: SecretsHandle { inner: secrets },
            http_out: HttpOutHandle { inner: http_out },
            fs: FsHandle { inner: fs },
            wall_clock: WallClockHandle { inner: wall_clock },
            tracing: TracingHandle { inner: tracing },
            warehouse_read: WarehouseReadHandle {
                inner: warehouse_read,
            },
            warehouse_write: WarehouseWriteHandle {
                inner: warehouse_write,
            },
            event_bus: EventBusHandle { inner: event_bus },
            dashboard: DashboardHandle { inner: dashboard },
            authz: AuthzHandle { inner: authz },
        }
    }

    /// Return a fresh `CtxInner` cloned from `self` with `caller` set
    /// to the supplied value (or cleared with `None`).
    ///
    /// Per-flavour dispatch loops call this once per inbound frame so
    /// every handler invocation sees its own immutable view of the
    /// requesting principal. Cheap — every field is `Arc`-backed.
    pub fn with_caller(mut self, caller: Option<CallerIdentity>) -> Self {
        self.caller = caller.map(Arc::new);
        self
    }

    /// Returns the always-present event sender.
    pub fn events(&self) -> &EventSender {
        &self.events
    }

    /// Returns the always-present cancellation handle.
    pub fn cancel(&self) -> &dyn Cancel {
        &*self.cancel
    }

    /// Returns the identity of the principal this invocation is being
    /// dispatched on behalf of, or `None` for a host-internal / system
    /// frame (health, init, lifecycle).
    ///
    /// Populated by the per-flavour entry-point glue from the inbound
    /// frame's `_meta.caller` sidecar. The reference is borrowed from
    /// the `Arc` the SDK clones per invocation — callers wanting to
    /// retain it past the handler's return should clone the value.
    pub fn caller(&self) -> Option<&CallerIdentity> {
        self.caller.as_deref()
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
    /// Borrow the warehouse-read handle (named by
    /// `requires!(warehouse_read)`).
    pub fn warehouse_read(&self) -> &WarehouseReadHandle {
        &self.warehouse_read
    }

    /// Borrow the warehouse-write handle (granted by
    /// `requires!(warehouse_write)`).
    pub fn warehouse_write(&self) -> &WarehouseWriteHandle {
        &self.warehouse_write
    }
    /// Borrow the event-bus handle (named by `requires!(event_bus)`).
    pub fn event_bus(&self) -> &EventBusHandle {
        &self.event_bus
    }
    /// Borrow the dashboard handle (named by `requires!(dashboard)`).
    pub fn dashboard(&self) -> &DashboardHandle {
        &self.dashboard
    }
    /// Borrow the authz handle (named by `requires!(authz)`).
    pub fn authz(&self) -> &AuthzHandle {
        &self.authz
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

pub use private::{AuthzBackend, DashboardBackend, EventBusBackend, FsBackend, HttpOutBackend, SecretsBackend, TracingBackend, WallClockBackend, WarehouseReadBackend, WarehouseWriteBackend};

mod private {
    /// Host-side backing for [`super::SecretsHandle`].
    pub trait SecretsBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Fetch a secret by name.
        fn get(&self, name: &str) -> starter_ext_spi::Result<String>;
    }

    /// Host-side backing for [`super::HttpOutHandle`].
    pub trait HttpOutBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Issue an outbound HTTP request encoded as JSON.
        fn request(&self, req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value>;
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

    /// Host-side backing for [`super::WarehouseReadHandle`].
    pub trait WarehouseReadBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Execute a named-template query.
        fn query(
            &self,
            template: &str,
            params: serde_json::Value,
        ) -> starter_ext_spi::Result<Vec<super::Row>>;

        /// Count the rows the template would yield.
        fn count(
            &self,
            template: &str,
            params: serde_json::Value,
        ) -> starter_ext_spi::Result<u64>;

        /// Look up a template's catalog entry.
        fn describe(
            &self,
            template: &str,
        ) -> starter_ext_spi::Result<Option<super::TemplateSpec>>;
    }

    /// Host-side backing for [`super::WarehouseWriteHandle`].
    pub trait WarehouseWriteBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Insert `rows` into `table`. Returns the engine's
        /// acknowledged row count.
        fn insert(
            &self,
            table: &str,
            rows: Vec<super::Row>,
        ) -> starter_ext_spi::Result<u64>;
    }

    /// Host-side backing for [`super::EventBusHandle`].
    pub trait EventBusBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Publish `payload` on `topic`.
        fn publish(
            &self,
            topic: &str,
            payload: serde_json::Value,
        ) -> starter_ext_spi::Result<()>;
    }

    /// Host-side backing for [`super::DashboardHandle`].
    pub trait DashboardBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Read the SDUI page body for `page_id` under the caller's
        /// bound tenancy.
        fn read(&self, page_id: &str) -> starter_ext_spi::Result<serde_json::Value>;
        /// Overwrite the SDUI page body for `page_id` under the
        /// caller's bound tenancy.
        fn write(
            &self,
            page_id: &str,
            body: serde_json::Value,
        ) -> starter_ext_spi::Result<()>;
    }

    /// Host-side backing for [`super::AuthzHandle`].
    pub trait AuthzBackend: std::fmt::Debug + Send + Sync + 'static {
        /// Evaluate `action` on `resource` for the bound principal.
        fn check(&self, action: &str, resource: &str) -> starter_ext_spi::Result<bool>;
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

    #[test]
    fn with_caller_sets_and_clears_the_field() {
        // Build a CtxInner via the stub backends from process.rs would
        // be circular — construct a bare one inline.
        use crate::ctx::private::*;
        #[derive(Debug)]
        struct Noop;
        impl SecretsBackend for Noop {
            fn get(&self, _: &str) -> starter_ext_spi::Result<String> {
                Ok(String::new())
            }
        }
        impl HttpOutBackend for Noop {
            fn request(&self, _: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
                Ok(serde_json::Value::Null)
            }
        }
        impl FsBackend for Noop {
            fn read(&self, _: &str) -> starter_ext_spi::Result<Vec<u8>> {
                Ok(Vec::new())
            }
        }
        impl WallClockBackend for Noop {
            fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
                Ok(0)
            }
        }
        impl TracingBackend for Noop {
            fn event(&self, _: &str, _: &str, _: serde_json::Value) {}
        }
        impl WarehouseReadBackend for Noop {
            fn query(
                &self,
                _: &str,
                _: serde_json::Value,
            ) -> starter_ext_spi::Result<Vec<Row>> {
                Ok(Vec::new())
            }
            fn count(&self, _: &str, _: serde_json::Value) -> starter_ext_spi::Result<u64> {
                Ok(0)
            }
            fn describe(
                &self,
                _: &str,
            ) -> starter_ext_spi::Result<Option<TemplateSpec>> {
                Ok(None)
            }
        }
        impl WarehouseWriteBackend for Noop {
            fn insert(&self, _: &str, _: Vec<Row>) -> starter_ext_spi::Result<u64> {
                Ok(0)
            }
        }
        impl EventBusBackend for Noop {
            fn publish(&self, _: &str, _: serde_json::Value) -> starter_ext_spi::Result<()> {
                Ok(())
            }
        }
        impl DashboardBackend for Noop {
            fn read(&self, _: &str) -> starter_ext_spi::Result<serde_json::Value> {
                Ok(serde_json::Value::Null)
            }
            fn write(&self, _: &str, _: serde_json::Value) -> starter_ext_spi::Result<()> {
                Ok(())
            }
        }
        impl AuthzBackend for Noop {
            fn check(&self, _: &str, _: &str) -> starter_ext_spi::Result<bool> {
                Ok(false)
            }
        }
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let inner = CtxInner::new(
            tx,
            Arc::new(NeverCancel),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
            Arc::new(Noop),
        );
        assert!(inner.caller().is_none(), "fresh CtxInner has no caller");
        let stamped = inner.clone().with_caller(Some(CallerIdentity {
            tenant_id: Some("t-1".into()),
            request_id: "r-1".into(),
            ..Default::default()
        }));
        let c = stamped.caller().expect("caller present after with_caller");
        assert_eq!(c.tenant_id.as_deref(), Some("t-1"));
        assert_eq!(c.request_id, "r-1");
        // Original inner is unchanged (with_caller takes self by value
        // but Clone is cheap).
        assert!(inner.caller().is_none());
        // Clearing.
        let cleared = stamped.with_caller(None);
        assert!(cleared.caller().is_none());
    }
}
