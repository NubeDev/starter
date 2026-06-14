//! Live-stream DTOs — create a subscription, then connect to its SSE feed.

mod create;
mod event;

pub use create::{CreateStreamRequest, CreateStreamResponse};
pub use event::StreamEvent;
