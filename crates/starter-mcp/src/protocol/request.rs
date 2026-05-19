//! Incoming JSON-RPC request.

use serde::Deserialize;

/// One JSON-RPC request frame.
///
/// `id` is `None` for notifications (per JSON-RPC 2.0); the server
/// must not respond to those.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Always `"2.0"`. Validated at parse time.
    pub jsonrpc: String,

    /// MCP method name (`"tools/list"`, `"tools/call"`, …).
    pub method: String,

    /// Method-specific parameters.
    #[serde(default)]
    pub params: serde_json::Value,

    /// Request id. `None` ⇒ notification (no response expected).
    pub id: Option<serde_json::Value>,
}
