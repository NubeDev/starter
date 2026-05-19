//! Server-Sent-Events helpers. Wraps `axum::response::sse` with the
//! two patterns starter consumers most often need: keep-alive +
//! adapt-from-stream.

mod from_stream;
mod keep_alive;

pub use from_stream::from_stream;
pub use keep_alive::keep_alive;
