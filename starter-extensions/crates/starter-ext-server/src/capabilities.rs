//! Capability-backend factory for adapter `Ctx` construction.
//!
//! `starter-ext-sdk` defines each capability's *trait* (e.g.
//! [`WarehouseReadBackend`], [`EventBusBackend`]). The substrate
//! ships a `Stub<Cap>` impl for each one — every adapter
//! currently constructs those stubs inline inside its `build_ctx`
//! helper. That keeps the substrate self-contained but means the
//! host has no seam through which to inject *real* backends
//! (e.g. the rubix-side `RubixWarehouseReadBackend` that binds
//! tenancy from `ctx.caller()`).
//!
//! This module is that seam.
//!
//! ## Shape
//!
//! [`CapabilityFactory`] is a small trait with one accessor per
//! capability the SDK exposes. Each accessor takes a
//! [`CallerIdentity`] (`None` for system / host-internal frames)
//! and returns the per-call backend the SDK will wrap in its
//! `*Handle`. The host is free to:
//!
//! - hand back a tenant-clamped backend (rubix-side
//!   `RubixWarehouseReadBackend`),
//! - hand back a shared singleton (an event bus backend that
//!   namespaces publishes by caller),
//! - hand back a stub that refuses the call
//!   ([`Error::Capability`]) — the same behaviour the substrate
//!   ships out of the box.
//!
//! ## What this slice ships
//!
//! - The trait + a [`StubCapabilityFactory`] that returns the same
//!   capability-not-wired stubs the dispatcher used to construct
//!   inline. Existing tests keep passing without changing a line.
//! - A new builder on [`super::BuiltinRestDispatcher`]:
//!   `with_capability_factory(factory)`. Hosts opt-in by passing
//!   their own factory; everyone else keeps the stubs.
//!
//! ## What this slice does *not* ship
//!
//! - **Caller extraction.** The dispatcher does not yet pull a
//!   [`CallerIdentity`] off the inbound HTTP request, so every
//!   factory call here receives `None`. A host factory that
//!   refuses on `None` (the rubix-agent shape) will surface
//!   `Error::Capability` on every call until caller extraction
//!   lands — that is the intentional, fail-closed staging:
//!   nothing tenant-scoped accidentally serves the wrong tenant
//!   while the plumbing is half-done.
//! - **Process / WASM dispatchers.** Their backends still live in
//!   `starter-ext-sdk::process` / `::wasm` and continue to be
//!   stubs. Wiring those goes through the supervisor's JSON-RPC
//!   host method handlers, not through this trait.

use std::sync::Arc;

use starter_ext_sdk::ctx::{
    AuthzBackend, DashboardBackend, EventBusBackend, WarehouseReadBackend, WarehouseWriteBackend,
};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::warehouse::{Row, TemplateSpec};
use starter_ext_spi::{Error, ExtensionId, Result};

/// One factory call returns the per-caller bundle of backends the
/// SDK will wrap in its `*Handle`s for a single `Ctx`.
///
/// Hosts implement this once and hand the resulting `Arc` to the
/// dispatcher via
/// [`super::BuiltinRestDispatcher::with_capability_factory`].
/// The dispatcher passes:
///
/// - `extension` — the id of the extension whose `Ctx` is being
///   built. Hosts use it as the publish-side namespace for the
///   event bus (an extension may only publish on topics under its
///   own id), as the lookup key for per-extension manifest
///   grants, and for log/trace attribution.
/// - `caller` — the principal the inbound HTTP frame was
///   dispatched on behalf of (`None` for system / host-internal
///   frames — see module docs).
pub trait CapabilityFactory: Send + Sync + 'static {
    /// Backend for `ctx.warehouse_read()`.
    fn warehouse_read(
        &self,
        extension: &ExtensionId,
        caller: Option<&CallerIdentity>,
    ) -> Arc<dyn WarehouseReadBackend>;

    /// Backend for `ctx.event_bus()`.
    fn event_bus(
        &self,
        extension: &ExtensionId,
        caller: Option<&CallerIdentity>,
    ) -> Arc<dyn EventBusBackend>;

    /// Backend for `ctx.warehouse_write()`. Default returns a
    /// fail-closed stub so hosts that haven't wired the write path
    /// yet keep compiling — same shape as `dashboard` / `authz`.
    fn warehouse_write(
        &self,
        _extension: &ExtensionId,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn WarehouseWriteBackend> {
        Arc::new(StubWarehouseWrite)
    }

    /// Backend for `ctx.dashboard()`. Default returns a fail-closed
    /// stub so hosts that haven't wired Row-5 yet keep compiling.
    fn dashboard(
        &self,
        _extension: &ExtensionId,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn DashboardBackend> {
        Arc::new(StubDashboard)
    }

    /// Backend for `ctx.authz()`. Default returns a fail-closed
    /// stub so hosts that haven't wired Row-5 yet keep compiling.
    fn authz(
        &self,
        _extension: &ExtensionId,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn AuthzBackend> {
        Arc::new(StubAuthz)
    }
}

/// Default [`CapabilityFactory`] returning capability-not-wired
/// stubs for every backend. Behaviour matches what the dispatcher
/// constructed inline before the factory landed — no test
/// observable change.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubCapabilityFactory;

impl CapabilityFactory for StubCapabilityFactory {
    fn warehouse_read(
        &self,
        _extension: &ExtensionId,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn WarehouseReadBackend> {
        Arc::new(StubWarehouseRead)
    }

    fn event_bus(
        &self,
        _extension: &ExtensionId,
        _caller: Option<&CallerIdentity>,
    ) -> Arc<dyn EventBusBackend> {
        Arc::new(StubEventBus)
    }
}

// ---------------------------------------------------------------------------
// Internal stub backends (kept here so the dispatcher module is no
// longer the source of these — single owner per capability shape).
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct StubWarehouseRead;

impl WarehouseReadBackend for StubWarehouseRead {
    fn query(&self, _template: &str, _params: serde_json::Value) -> Result<Vec<Row>> {
        Err(Error::capability(
            "warehouse_read not wired: install a host CapabilityFactory",
        ))
    }
    fn count(&self, _template: &str, _params: serde_json::Value) -> Result<u64> {
        Err(Error::capability(
            "warehouse_read not wired: install a host CapabilityFactory",
        ))
    }
    fn describe(&self, _template: &str) -> Result<Option<TemplateSpec>> {
        Err(Error::capability(
            "warehouse_read not wired: install a host CapabilityFactory",
        ))
    }
}

#[derive(Debug)]
struct StubEventBus;

impl EventBusBackend for StubEventBus {
    fn publish(&self, _topic: &str, _payload: serde_json::Value) -> Result<()> {
        Err(Error::capability(
            "event_bus not wired: install a host CapabilityFactory",
        ))
    }
}

#[derive(Debug)]
struct StubWarehouseWrite;

impl WarehouseWriteBackend for StubWarehouseWrite {
    fn insert(&self, _table: &str, _rows: Vec<Row>) -> Result<u64> {
        Err(Error::capability(
            "warehouse_write not wired: install a host CapabilityFactory",
        ))
    }
}

#[derive(Debug)]
struct StubDashboard;

impl DashboardBackend for StubDashboard {
    fn read(&self, _page_id: &str) -> Result<serde_json::Value> {
        Err(Error::capability(
            "dashboard not wired: install a host CapabilityFactory",
        ))
    }
    fn write(&self, _page_id: &str, _body: serde_json::Value) -> Result<()> {
        Err(Error::capability(
            "dashboard not wired: install a host CapabilityFactory",
        ))
    }
}

#[derive(Debug)]
struct StubAuthz;

impl AuthzBackend for StubAuthz {
    fn check(&self, _action: &str, _resource: &str) -> Result<bool> {
        Err(Error::capability(
            "authz not wired: install a host CapabilityFactory",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_factory_warehouse_refuses_with_capability() {
        let f = StubCapabilityFactory;
        let ext = ExtensionId::new("com.acme.cap").unwrap();
        let backend = f.warehouse_read(&ext, None);
        let err = backend
            .query("anything", serde_json::Value::Null)
            .expect_err("stub must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[test]
    fn stub_factory_event_bus_refuses_with_capability() {
        let f = StubCapabilityFactory;
        let ext = ExtensionId::new("com.acme.cap").unwrap();
        let backend = f.event_bus(&ext, None);
        let err = backend
            .publish("any.topic", serde_json::Value::Null)
            .expect_err("stub must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    /// A factory that hands back distinguishable backends so the
    /// integration test below can assert `with_capability_factory`
    /// actually plumbs the override into `build_ctx`.
    #[derive(Debug)]
    struct MarkerFactory;

    #[derive(Debug)]
    struct MarkerWarehouse;
    impl WarehouseReadBackend for MarkerWarehouse {
        fn query(&self, _t: &str, _p: serde_json::Value) -> Result<Vec<Row>> {
            Err(Error::capability("marker-warehouse-installed"))
        }
        fn count(&self, _t: &str, _p: serde_json::Value) -> Result<u64> {
            Err(Error::capability("marker-warehouse-installed"))
        }
        fn describe(&self, _t: &str) -> Result<Option<TemplateSpec>> {
            Err(Error::capability("marker-warehouse-installed"))
        }
    }

    #[derive(Debug)]
    struct MarkerEventBus;
    impl EventBusBackend for MarkerEventBus {
        fn publish(&self, _t: &str, _p: serde_json::Value) -> Result<()> {
            Err(Error::capability("marker-event-bus-installed"))
        }
    }

    impl CapabilityFactory for MarkerFactory {
        fn warehouse_read(
            &self,
            _extension: &ExtensionId,
            _caller: Option<&CallerIdentity>,
        ) -> Arc<dyn WarehouseReadBackend> {
            Arc::new(MarkerWarehouse)
        }
        fn event_bus(
            &self,
            _extension: &ExtensionId,
            _caller: Option<&CallerIdentity>,
        ) -> Arc<dyn EventBusBackend> {
            Arc::new(MarkerEventBus)
        }
    }

    /// Scaffolding so `super::dispatcher` tests can use the same marker
    /// factory without copy-pasting the impl. Currently unused — kept as
    /// the seam a later phase's dispatcher tests plug into.
    #[allow(dead_code)]
    pub(crate) fn marker_factory() -> Arc<dyn CapabilityFactory> {
        Arc::new(MarkerFactory)
    }
}
