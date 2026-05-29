//! Host backends for the extension-substrate capability handles.
//!
//! Per row 5 of
//! [`docs/scope/extensions-north-star/README.md`](../../../../docs/scope/extensions-north-star/README.md):
//! these are the rubix-side concrete impls of the SDK-defined
//! [`WarehouseReadBackend`][starter_ext_sdk::ctx::WarehouseReadBackend]
//! and [`EventBusBackend`][starter_ext_sdk::ctx::EventBusBackend]
//! traits. Extensions running in the host's address space (builtin
//! flavour) reach these via `ctx.warehouse_read()` /
//! `ctx.event_bus()`; the per-call factory binds the calling
//! principal's `tenant_id` and the extension's reverse-DNS
//! namespace before the handler observes the handle, closing the
//! soft trust boundary identified in Appendix A of the
//! extension-architecture-north-star proposal.
//!
//! Process-flavour and WASM-flavour extensions still get the
//! `Stub<Cap>` impls from `starter-ext-sdk` for v0.1 — plumbing
//! these backends through the JSON-RPC loopback is a follow-up
//! slice (it requires the host side of the `warehouse.query` /
//! `event_bus.publish` host methods).

pub mod backends;
pub mod caller_identity;
pub mod cleanup;
pub mod dashboard_authz;
pub mod event_bus;
pub mod host_methods;
pub mod rest_dispatcher;
pub mod secrets_store;
pub mod warehouse_write;

pub use backends::{RubixCapabilityFactory, RubixWarehouseReadBackend};
pub use caller_identity::with_caller_identity;
pub use cleanup::{SkillCleanupProvider, WarehouseCleanupProvider};
pub use dashboard_authz::{RubixAuthzBackend, RubixDashboardBackend};
pub use event_bus::{RubixEventBus, RubixEventBusBackend};
pub use host_methods::RubixHostMethods;
pub use rest_dispatcher::{CompositeRestDispatcher, EXTENSION_REST_REQUEST_TIMEOUT};
pub use secrets_store::{pick_default as pick_default_secret_store, EnvSecretStore};
pub use warehouse_write::{full_table_name, sanitize_extension_id, RubixWarehouseWriteBackend};
