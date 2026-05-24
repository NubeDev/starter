//! `ai-agent` node kind.
//!
//! A thin [`starter_flow_spi::node::NodeBehavior`] wrapper around
//! [`starter_ai_agent::AgentLoop`]. The flow engine resolves the
//! `ai-agent` kind to this behaviour; on invoke it reads the prompt
//! from the input slot, optionally filters its tool set by the
//! `allowed_tools` config slot, builds a fresh `AgentLoop`, calls
//! `.run()`, and writes the model's reply to the `out` slot.

pub mod node;

pub use node::{AiAgentNode, IN_SLOT_PROMPT, KIND_ID, OUT_SLOT_REPLY, SETTINGS_ALLOWED_TOOLS};
