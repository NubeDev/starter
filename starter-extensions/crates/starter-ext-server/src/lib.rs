//! # starter-ext-server
//!
//! Axum integration for the `starter-extensions` workspace — the admin
//! slice. Turns a sealed [`ExtensionRegistry`] plus a map of running
//! [`SupervisorHandle`]s into HTTP routes mountable into the parent
//! workspace's `starter-server::ServerBuilder` (same pattern as
//! `starter-mcp`).
//!
//! This crate ships **only the admin surface**:
//!
//! - `GET  /extensions`            — list every extension with id, version,
//!   lifecycle state, restart count.
//! - `GET  /extensions/<id>`       — full record including manifest,
//!   capability grants, current state, and capability-violation counter.
//! - `GET  /extensions/<id>/events` — paginated JSON snapshot of the
//!   supervisor's event ring; upgrades to SSE for a live tail when the
//!   client sends `Accept: text/event-stream` (or `?stream=1`).
//! - `POST /extensions/<id>/enable` / `disable` — runtime toggle,
//!   persisted through an [`EnablementStore`] (one DB row per id per
//!   SCOPE Decision). Disable sends shutdown to the supervisor; enable
//!   re-spawns process-flavour records via [`SupervisorFactory`].
//! - `GET  /extensions/<id>/ui/*`  — serves the extension's UI bundle
//!   directory for Module-Federation loading. Strong ETags from SHA-256
//!   of the file bytes; cached in memory keyed by canonical path.
//!
//! All four mutating / privileged endpoints are gated by
//! `with_principal` → `with_role(Role::Admin)` from `starter-server`.
//! The REST *contribution* surface (mounting `contributes.rest` entries
//! from each extension's manifest) lives in Adapter Phase 5 — explicitly
//! out of scope for this crate today.
//!
//! ## Composition
//!
//! ```ignore
//! use std::sync::Arc;
//! use starter_ext_server::{ExtensionAdmin, AdminRouterOptions};
//!
//! let admin = ExtensionAdmin::builder(registry)
//!     .with_enablement_store(Arc::new(MyDbStore::new(pool)))
//!     .with_supervisor_factory(Arc::new(DefaultSupervisorFactory))
//!     .build();
//!
//! let router = starter_ext_server::router(admin.clone(), AdminRouterOptions {
//!     authenticator: Some(my_authenticator),
//!     ..Default::default()
//! });
//! let app = ServerBuilder::new(state).merge_router(router).build();
//! ```
//!
//! With no `Authenticator` (test mode) the middleware is omitted and the
//! routes are unauthenticated — this is for `TestApp` only and must not
//! be wired into a production binary.
//!
//! [`ExtensionRegistry`]: starter_ext_host::ExtensionRegistry
//! [`SupervisorHandle`]: starter_ext_supervisor::SupervisorHandle

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod admin;
mod capabilities;
mod etag;
mod events;
mod factory;
mod i18n;
mod issues;
mod lifecycle;
mod process;
pub mod rest;
mod router;
mod routes;
mod store;
mod ui;

pub use admin::{ExtensionAdmin, ExtensionAdminBuilder};
pub use capabilities::{CapabilityFactory, StubCapabilityFactory};
pub use factory::{
    DefaultSupervisorFactory, SupervisorFactory, SupervisorFactoryError, WithHostMethodsFactory,
};
pub use rest::{
    rest_router, BuiltinRestDispatcher, DispatchError, DispatcherCache, KindCacheRegistry,
    NotWiredDispatcher, OrphanSidecar, ProcessRestDispatcher, RestBuildError, RestDispatcher,
    RestRouterOptions, SchemaCheck, SidecarLoadError, StreamResponse,
};
pub use router::{router, router_with_auth, AdminRouterOptions};
pub use store::{EnablementState, EnablementStore, InMemoryEnablementStore, StoreError};
