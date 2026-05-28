//! Map of tool-name → boxed [`Tool`]. Built at startup, immutable
//! during the server's lifetime. Also owns the parallel
//! [`PromptRegistry`] so a single value carries both surfaces
//! into [`crate::server::dispatch`].

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::tool::{Tool, ToolDefinition};

use super::prompt_registry::{Prompt, PromptRegistry};

/// Mutable builder + immutable runtime registry of tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    prompts: PromptRegistry,
}

impl ToolRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool. Last-write-wins on duplicate names — pass
    /// each name once at startup.
    pub fn register<T: Tool>(mut self, tool: T) -> Self {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), Arc::new(tool));
        self
    }

    /// Register an already-`Arc`-wrapped tool. Same shape as
    /// [`Self::register`] but for callers that need to share the
    /// same `Arc<dyn Tool>` instance across multiple registries
    /// (e.g. rubix surfaces the same tool list to both its REST
    /// router and its MCP catalogue).
    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
        self
    }

    /// Register a prompt for the MCP `prompts` capability.
    pub fn register_prompt<P: Prompt>(mut self, prompt: P) -> Self {
        self.prompts = self.prompts.register(prompt);
        self
    }

    /// Register an already-`Arc`-wrapped prompt.
    pub fn register_prompt_arc(mut self, prompt: Arc<dyn Prompt>) -> Self {
        self.prompts = self.prompts.register_arc(prompt);
        self
    }

    /// All registered tool definitions, in registration order.
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Borrow the embedded [`PromptRegistry`].
    pub fn prompts(&self) -> &PromptRegistry {
        &self.prompts
    }
}
