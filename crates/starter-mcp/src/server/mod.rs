//! Server loop + per-method dispatch. One file per concern.

mod dispatch;
mod stdio_loop;
mod stdio_loop_locale;
mod stdio_ndjson_loop;

#[cfg(feature = "http")]
mod http;

pub use dispatch::dispatch;
pub use stdio_loop::run_stdio;
pub use stdio_ndjson_loop::run_stdio_ndjson;

#[cfg(feature = "http")]
pub use http::{mcp_router, McpHttpOptions};
