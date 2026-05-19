//! The `Tool` trait. Consumers implement this; `starter-mcp`
//! collects implementations and exposes them over the MCP
//! protocol.

use async_trait::async_trait;

use crate::error::Result;

use super::definition::ToolDefinition;

/// A callable tool. Implementations carry their own state
/// (database handles, HTTP clients, etc.) and are typically
/// registered with the MCP server at startup.
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// Return the static metadata advertised to MCP clients.
    fn definition(&self) -> ToolDefinition;

    /// Invoke the tool with the caller-supplied input.
    ///
    /// Input is the raw JSON value the client sent. The transport
    /// will already have validated it against `definition().input_schema`,
    /// so implementations can deserialize without re-checking shape.
    async fn invoke(&self, input: serde_json::Value) -> Result<serde_json::Value>;
}
