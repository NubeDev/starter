//! Server-Driven UI — opt-in HTTP routes crate.
//!
//! Phase 5 of the SDUI port (DOCS/frontend/sdui/SCOPE.md). Mounts
//! three endpoints under `/api/v1/ui`:
//!
//! - `POST /resolve` — resolve a `(page_ref, target_ref)` to a
//!   [`starter_ui_ir::ComponentTree`] plus the per-resolve
//!   subscription plan.
//! - `POST /action`  — dispatch a named handler from the
//!   [`HandlerRegistry`]; response is the discriminated
//!   [`starter_ui_ir::ActionResponse`] union per **R5**.
//! - `GET  /table`   — paginated table source backed by the
//!   [`QueryEngine`] trait per **R6**.
//!
//! # Why this is a separate crate (D4 / M6)
//!
//! Cargo features on `starter-server` cannot prevent the underlying
//! `starter-ui-ir` / `starter-ui-bindings` crates from being built
//! — workspace crates compile if anything in the workspace depends
//! on them. The only honest opt-out is at the **consumer's
//! `Cargo.toml`**: a consumer that wants the SDUI routes adds
//! `starter-sdui-routes = "0.1"` and calls [`sdui_router`]; a
//! consumer that wants only the IR + builder never sees axum, the
//! binding engine, or any of this crate's machinery.
//!
//! `starter-server` therefore does **not** depend on this crate.
//! See `DOCS/frontend/sdui/DIVERGENCE.md` § D4.
//!
//! # DoS limits (R8)
//!
//! Each request travels through [`limits`] before it touches a
//! handler. A violation produces an `HTTP 413 Payload Too Large`
//! whose body carries a stable `what:` tag (see
//! [`limits::WhatTag`]) — the integration tests in
//! `tests/limits_413.rs` pin one tag per limit. The *enforcement* is
//! covered; the *limit values* themselves are inherited from Rubix
//! and re-measured the first time a consumer reports them.
//!
//! # Capability handshake threat model (R7)
//!
//! `renderer_id` for [`starter_ui_ir::Component::Custom`] is treated
//! as **public**. The capability filter is a *vocabulary* check
//! ("does this client know how to render this id") — never an
//! *authorisation* check ("is this user allowed to see this data").
//! Auth runs at the handler boundary (R5) and at the resolve
//! boundary, both before any `custom` node is constructed. A
//! handler emitting `custom.props` is responsible for ensuring
//! those props are appropriate for the [`Principal`] the resolve
//! was issued against.
//!
//! See SCOPE.md § R7 for the full threat model.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cache_integration;
pub mod capability;
pub mod chart_resolve;
pub mod error;
pub mod handler;
pub mod limits;
pub mod page;
pub mod query;
mod router;
pub mod routes;
pub mod state;

pub use capability::{CapabilityFilter, ClientCapabilities, SUPPORTED_IR_VERSION};
pub use chart_resolve::{resolve_chart_sources, AnalyticsBridge, AnalyticsBridgeRef};
pub use error::{SduiError, WhatTag};
pub use handler::{
    ActionFn, ActionFuture, HandlerContext, HandlerNotFound, HandlerRegistry, Principal,
};
pub use page::{InMemoryPageProvider, PageProvider, PageRef};
pub use query::{InMemoryQueryEngine, QueryEngine, QueryRequest, QueryResponse, TableRow};
pub use router::sdui_router;
pub use routes::{action::ActionBody, resolve::ResolveRequest, resolve::ResolveResponse};
pub use state::{SduiState, SduiStateBuilder};
