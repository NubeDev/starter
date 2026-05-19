//! MCP-tool shape. `starter-mcp` reads this trait; consumers
//! implement it. The trait lives here so transports beyond MCP
//! (e.g. an HTTP "actions" endpoint) can reuse the same shape.

mod definition;
mod kind;

pub use definition::ToolDefinition;
pub use kind::Tool;
