//! Consumer-owned MCP tool. Implements the `Tool` trait from
//! `starter-spi` and registers into starter's `ToolRegistry`.
//! Starter doesn't need to know it exists — the registry is open.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result as SpiResult;

use crate::domain::NoteStore;

pub struct NoteSearchTool {
    pub store: Arc<NoteStore>,
}

#[async_trait]
impl Tool for NoteSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "note_search".into(),
            description: "Search notes by case-sensitive substring on the body field.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": { "type": "string", "description": "Substring to match." }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| starter_spi::Error::Invalid {
                message: "missing 'query' string".into(),
            })?;
        let notes = self
            .store
            .search(query)
            .await
            .map_err(|e| starter_spi::Error::Internal { source: Box::new(e) })?;
        Ok(serde_json::to_value(&notes).unwrap_or(Value::Null))
    }
}
