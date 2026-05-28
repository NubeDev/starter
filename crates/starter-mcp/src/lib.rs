//! # starter-mcp
//!
//! MCP (Model Context Protocol) server scaffold. The consumer
//! registers `starter_spi::tool::Tool` implementations; this crate
//! handles JSON-RPC framing, dispatch, and (optionally) bearer auth.
//!
//! Two transports ship in 0.1:
//!
//! - **stdio** (default) — [`run_stdio`] reads framed JSON-RPC from
//!   stdin and writes responses to stdout. The MCP norm for desktop
//!   tools (Claude Desktop, Codex CLI, etc.).
//! - **HTTP** (`feature = "http"`) — [`server::mcp_router`] returns an
//!   `axum::Router` exposing `POST /mcp`. Merge into the consumer's
//!   `starter-server`. Optional [`server::McpHttpOptions::with_auth`]
//!   enforces `Authorization: Bearer …`.
//!
//! SSE (Streamable HTTP) progress events are a v0.2 follow-up; the
//! request/response half is complete today.
//!
//! - [`server`] — dispatch + transports.
//! - [`registry`] — `ToolRegistry`, consumer adds tools at startup.
//! - [`protocol`] — JSON-RPC envelope types.
//! - [`testing`] — in-memory transport pair (`feature = "testing"`):
//!   [`testing::pair`] returns a `(client, server)` that round-trips real
//!   JSON-RPC frames through the same dispatch core HTTP and stdio use.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod locale_local;
pub mod principal_local;
pub mod protocol;
pub mod registry;
pub mod server;

pub use locale_local::{current_locale, with_locale};
pub use principal_local::{current_principal, with_principal};

#[cfg(feature = "skills")]
pub mod skills_bridge;

#[cfg(feature = "testing")]
pub mod testing;

pub use registry::ToolRegistry;
pub use server::run_stdio;

#[cfg(feature = "http")]
pub use server::{mcp_router, McpHttpOptions};
