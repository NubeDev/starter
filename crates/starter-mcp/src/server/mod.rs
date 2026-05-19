//! Stdio server loop + per-method dispatch. One file per concern.

mod dispatch;
mod stdio_loop;

pub use dispatch::dispatch;
pub use stdio_loop::run_stdio;
