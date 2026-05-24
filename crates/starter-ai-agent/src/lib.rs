//! Runner-agnostic LLM ⇄ tool agent loop.
//!
//! Thin v0 owns one shape: drive a [`starter_spi::ai::AiRunner`] with
//! a user prompt and a fixed [`ToolSet`], dispatch any tool calls the
//! model emits back through the same set, then call the runner one
//! more time so it can produce a final reply that consumes the tool
//! outputs. Multi-turn session persistence, cost caps, cancellation,
//! streaming, and skill enforcement are explicitly deferred — see
//! [LONG-TERM.md](../../LONG-TERM.md).

pub mod agent_loop;
pub mod error;
pub mod prompt;
pub mod testing;
pub mod tool_set;

pub use agent_loop::AgentLoop;
pub use error::AgentError;
pub use tool_set::ToolSet;
