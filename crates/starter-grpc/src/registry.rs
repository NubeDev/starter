//! Map of tool-name → boxed [`Tool`]. Built at startup, immutable
//! during the server's lifetime.
//!
//! Identical in shape to [`starter_mcp::ToolRegistry`] — kept
//! independent so a consumer who wants gRPC without MCP does not
//! transitively pull `starter-mcp`'s JSON-RPC stack. The two
//! registries hold the same `Arc<dyn Tool>` values; a consumer that
//! wants both surfaces typically builds the `Vec<Arc<dyn Tool>>` once
//! and `register`s each tool with both registries.

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
    /// each name once at startup. Mirrors `starter_mcp::ToolRegistry`
    /// so a consumer building both registries follows the same rule.
    pub fn register<T: Tool>(mut self, tool: T) -> Self {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), Arc::new(tool));
        self
    }

    /// Register a pre-boxed tool. Useful when the same tool is
    /// surfaced on multiple transports — build the `Arc` once,
    /// hand it to MCP and gRPC.
    pub fn register_arc(mut self, tool: Arc<dyn Tool>) -> Self {
        let def = tool.definition();
        self.tools.insert(def.name.clone(), tool);
        self
    }

    /// All registered tool definitions.
    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}
