//! Project the tool registry into [`RegistryItem`]s.
//!
//! Each tool's [`ToolDefinition`](starter_spi::tool::ToolDefinition)
//! drops into the wire envelope unchanged: `name → id`,
//! `description → summary`, `input_schema → input_schema`. The
//! per-kind metadata reports `mcp_compatible: true` for every
//! tool — every rubix tool surfaces over MCP today via the same
//! [`crate::registry::build_tool_registry`] seed.

use std::sync::Arc;

use rubix_spi::dto::admin::{ItemSource, RegistryItem};
use serde_json::json;
use starter_ext_host::ExtensionRegistry;
use starter_spi::tool::Tool;

use super::source::item_source;

/// Project every registered tool. Output order is the registry's
/// natural order; the paginator sorts before slicing.
pub fn tool_items(
    tools: &[Arc<dyn Tool>],
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> Vec<RegistryItem> {
    tools
        .iter()
        .map(|t| to_item(&**t, extensions))
        .collect()
}

/// Project a single tool. Surfaced so the per-id detail route can
/// reuse the same shape without re-walking the registry.
pub fn tool_to_item(
    tool: &dyn Tool,
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> RegistryItem {
    to_item(tool, extensions)
}

fn to_item(tool: &dyn Tool, extensions: Option<&Arc<ExtensionRegistry>>) -> RegistryItem {
    let def = tool.definition();
    let source: ItemSource = item_source(&def.name, extensions);
    let metadata = json!({
        "mcp_compatible": true,
    });
    RegistryItem::new(def.name.clone(), source)
        .with_label(def.name.clone())
        .with_summary(def.description)
        .with_input_schema(def.input_schema)
        .with_metadata(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::Value;
    use starter_spi::error::Result;
    use starter_spi::tool::ToolDefinition;

    struct Dummy;
    #[async_trait]
    impl Tool for Dummy {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "rubix.system.disk".into(),
                description: "Disk free check".into(),
                input_schema: json!({"type": "object"}),
            }
        }
        async fn invoke(&self, _input: Value) -> Result<Value> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn projects_tool_definition_into_item() {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(Dummy)];
        let items = tool_items(&tools, None);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "rubix.system.disk");
        assert_eq!(items[0].summary, "Disk free check");
        assert!(items[0].input_schema.is_some());
        assert_eq!(items[0].metadata["mcp_compatible"], Value::Bool(true));
        assert!(matches!(items[0].source, ItemSource::Builtin));
    }
}
