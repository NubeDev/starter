//! Map of tool-name → boxed [`Tool`]. Built at startup, immutable
//! during the server's lifetime.

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::tool::{Tool, ToolDefinition};

/// Mutable builder + immutable runtime registry of tools.
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
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

    /// All registered tool definitions, in registration order.
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}
