//! Read-only collection of tools the loop is allowed to dispatch to.

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::ai::ToolDef;
use starter_spi::tool::Tool;

/// Read-only view of the tools an [`crate::AgentLoop`] may invoke.
///
/// Constructed by the caller from the host's full tool registry —
/// typically after applying a per-skill allowlist — and handed to
/// the loop verbatim. The set is immutable for the lifetime of the
/// loop call; rotating tools mid-run is a future concern.
#[derive(Clone, Default)]
pub struct ToolSet {
    by_name: HashMap<String, Arc<dyn Tool>>,
}

impl ToolSet {
    /// Build a [`ToolSet`] from a list of tool handles. Last writer
    /// wins on duplicate names.
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        let mut by_name = HashMap::with_capacity(tools.len());
        for tool in tools {
            by_name.insert(tool.definition().name.clone(), tool);
        }
        Self { by_name }
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.by_name.get(name)
    }

    /// Number of tools in the set.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// `true` iff the set is empty.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Render the tools as the schema shape the runner expects.
    /// `ToolDefinition` and `ToolDef` carry the same three fields;
    /// this is the only translation between them.
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.by_name
            .values()
            .map(|t| {
                let d = t.definition();
                ToolDef {
                    name: d.name,
                    description: Some(d.description),
                    input_schema: d.input_schema,
                }
            })
            .collect()
    }
}
