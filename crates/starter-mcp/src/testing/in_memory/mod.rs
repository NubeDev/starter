//! A real bidirectional in-memory transport for `starter-mcp`.
//!
//! Lets tests drive a full `initialize` → `tools/list` → `tools/call`
//! round-trip without spawning HTTP listeners or shelling out to stdio.
//! Frames are serialised JSON-RPC just like the wire transports — only
//! the carrier changes — so the dispatch surface tested here is the
//! same surface consumers hit in production.
//!
//! The transport mirrors both wire transports' task-local plumbing:
//!
//! - Per-server-construction `Principal` binding, like the HTTP
//!   transport's `with_principal` wrap around `dispatch` (see
//!   `crate::server::http`).
//! - Per-session `LanguageTag` captured from
//!   `params._meta.acceptLanguage` on `initialize`, then held for every
//!   subsequent dispatch — exactly the stdio loop's convention (see
//!   `crate::server::stdio_loop`).
//!
//! Closes the Phase 2b U2 gap recorded in
//! `docs/design/starter-changes/README.md`.

mod client;
mod server;

pub use client::InMemoryClient;
pub use server::InMemoryServer;

use std::sync::Arc;

use crate::registry::ToolRegistry;

/// Build a paired in-memory transport. The returned [`InMemoryClient`]
/// sends JSON-RPC frames; the spawned [`InMemoryServer`] task dispatches
/// them through the same core as HTTP and stdio and returns responses on
/// the client's receive channel.
///
/// Dropping the client closes the server's inbound channel and the
/// dispatch task exits cleanly.
pub fn pair(registry: Arc<ToolRegistry>) -> (InMemoryClient, InMemoryServer) {
    InMemoryServer::spawn(registry, None)
}

/// Build a paired in-memory transport whose dispatch task runs with
/// `principal` bound on the [`crate::principal_local`] task-local.
/// Mirrors `crate::server::http::auth_layer`'s `with_principal` wrap so
/// tests can exercise `AuthzedToolBinding`-style wrappers without
/// standing up the HTTP stack.
pub fn pair_with_principal(
    registry: Arc<ToolRegistry>,
    principal: starter_spi::auth::Principal,
) -> (InMemoryClient, InMemoryServer) {
    InMemoryServer::spawn(registry, Some(principal))
}

/// Type alias the carriers share — one JSON-RPC frame per channel item.
pub(super) type Frame = String;
