//! # starter-flow-surfaces
//!
//! Flow ↔ Tool / Service wrappers per `DOCS/flow/scope/SCOPE.md` R8
//! and R9:
//!
//! - `FlowAsTool` — wraps a flow as `starter_spi::tool::Tool`. Makes
//!   every flow automatically MCP-callable, REST-callable,
//!   CLI-callable, and callable from another flow as a `tool-call`
//!   node.
//! - `FlowAsService` — wraps a flow as `starter_spi::service::Service`.
//!   Reads from an `EventSink`; invokes the flow per event.
//!
//! Phase 1 ships this crate as an empty skeleton. The wrappers land in
//! Phase 3 alongside persistence.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
