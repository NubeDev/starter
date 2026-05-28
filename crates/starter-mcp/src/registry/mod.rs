//! `ToolRegistry` — the collection of [`starter_spi::tool::Tool`]
//! implementations a server exposes. `PromptRegistry` is the
//! parallel surface for the MCP `prompts` capability; hosts that
//! map prompts to slash commands read it via `prompts/list`.

mod prompt_registry;
mod tool_registry;

pub use prompt_registry::{
    Prompt, PromptArgument, PromptDefinition, PromptMessage, PromptRegistry, PromptResponse,
    PromptRole,
};
pub use tool_registry::ToolRegistry;
