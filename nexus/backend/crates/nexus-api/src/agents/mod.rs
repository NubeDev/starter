//! Agent session runtime — the in-memory machinery that drives a session run
//! and feeds its SSE subscribers, separate from the persistence in nexus-store.

pub mod runtime;

pub use runtime::{PromptInputs, SessionRunner};
