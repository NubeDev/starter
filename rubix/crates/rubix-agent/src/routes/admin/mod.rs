//! `/api/v1/admin/*` — read-only registry projection routes.
//!
//! LAYER: transport (REST). Every handler extracts query
//! parameters, calls a `crate::admin::*_items()` projection,
//! applies pagination, and returns the envelope JSON. No
//! domain logic — see [docs/design/admin/](../../../../docs/design/admin/README.md).

mod errors;
mod extensions;
mod invoke;
mod invoke_stream;
mod nodes;
mod openapi;
mod overview;
mod query;
mod registry;
mod rules;
mod skills;
mod tables;
mod templates;
mod tools;

pub use invoke::admin_invoke_registrar;
pub use invoke_stream::admin_invoke_stream_registrar;
pub use openapi::admin_openapi_registrar;
pub use registry::admin_registrar;

use crate::admin::AdminState;
use axum::Router;

/// Backwards-compatible alias returning an `axum::Router` directly.
/// Used by integration tests that pre-date the registrar; new
/// call sites consume [`admin_registrar`] instead.
pub fn admin_router(state: AdminState) -> Router {
    admin_registrar(state).into_router()
}

/// Backwards-compatible alias for the invoke router.
pub fn admin_invoke_router(state: AdminState) -> Router {
    admin_invoke_registrar(state).into_router()
}
