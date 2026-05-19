//! Server loop + per-method dispatch. One file per concern.

mod dispatch;
mod stdio_loop;

#[cfg(feature = "http")]
mod http;

pub use dispatch::dispatch;
pub use stdio_loop::run_stdio;

#[cfg(feature = "http")]
pub use http::{mcp_router, McpHttpOptions};
