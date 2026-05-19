//! # starter-mcp
//!
//! MCP (Model Context Protocol) server scaffold. The consumer
//! registers `starter_spi::tool::Tool` implementations; this crate
//! handles the stdio loop, JSON-RPC framing, auth check, and
//! dispatch.
//!
//! Stdio transport only for v1 (SCOPE.md).
//!
//! - [`server`] — top-level run loop.
//! - [`registry`] — `ToolRegistry`, consumer adds tools at startup.
//! - [`protocol`] — JSON-RPC envelope types.
//! - [`testing`] — in-memory transport pair (`feature = "testing"`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod protocol;
pub mod registry;
pub mod server;

#[cfg(feature = "testing")]
pub mod testing;

pub use registry::ToolRegistry;
pub use server::run_stdio;
