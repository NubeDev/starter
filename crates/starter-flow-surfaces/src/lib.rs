//! # starter-flow-surfaces
//!
//! Flow ↔ Tool / Service wrappers per `DOCS/flow/scope/SCOPE.md` R8
//! and R9:
//!
//! - [`FlowAsTool`] — wraps a flow as `starter_spi::tool::Tool`. Makes
//!   every flow automatically MCP-callable, REST-callable,
//!   CLI-callable, and callable from another flow as a `tool-call`
//!   node.
//! - [`FlowAsService`] — wraps a flow as `starter_spi::service::Service`.
//!   Reads from an `EventSink`; invokes the flow per event.
//!
//! Phase 1 ships this crate as a public-API placeholder. The wrappers
//! land in Phase 3 alongside persistence. Per `CLAUDE.md` the bodies
//! and trait impls are deliberately absent (rather than `todo!()`) so
//! a half-built impl cannot escape into a consumer accidentally.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Wraps a flow as a `starter_spi::tool::Tool`.
///
/// Semantics are specified by `DOCS/flow/scope/SCOPE.md` §R8
/// "Nodes are not Tools — Tools are one node kind": the outside world
/// sees a single first-class `Tool`; internally the flow's `tool-call`
/// nodes are the only place where `Tool::call` actually fires. Wrapping
/// a flow this way makes it MCP-callable, REST-callable, CLI-callable,
/// and callable from another flow as a `tool-call` node — for free.
///
/// Fields land in Phase 3 (see §R8). The struct is intentionally empty
/// today so consumers can name the type in `where` bounds and re-exports
/// without inheriting a half-built impl.
pub struct FlowAsTool {
    // TODO Phase 3 — fields per R8
    // (flow handle / id, engine handle, input/output schema, name,
    // description, etc.). Left absent rather than `todo!()` so a
    // half-finished impl cannot leak into consumers.
}

/// Wraps a flow as a `starter_spi::service::Service`.
///
/// Semantics are specified by `DOCS/flow/scope/SCOPE.md` §R9
/// "Flows are first-class Tools and first-class Services": the service
/// reads from an `EventSink` and invokes the flow once per event,
/// giving every flow a "run on each X" deployment without bespoke
/// glue. The companion of `FlowAsTool`'s "callable" shape.
///
/// Fields land in Phase 3 (see §R9). The struct is intentionally empty
/// today so consumers can name the type in `where` bounds and re-exports
/// without inheriting a half-built impl.
pub struct FlowAsService {
    // TODO Phase 3 — fields per R9
    // (flow handle / id, engine handle, event-sink subscription,
    // service name, lifecycle hooks, etc.). Left absent rather than
    // `todo!()` so a half-finished impl cannot leak into consumers.
}
