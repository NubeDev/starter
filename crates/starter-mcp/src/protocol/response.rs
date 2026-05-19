//! Outgoing JSON-RPC response — success or error.

use serde::Serialize;

use super::error::RpcError;

/// One JSON-RPC response frame. Exactly one of `result` or `error`
/// is set.
#[derive(Debug, Serialize)]
pub struct Response {
    /// Always `"2.0"`.
    pub jsonrpc: &'static str,

    /// The originating request's id. Echoed verbatim.
    pub id: serde_json::Value,

    /// Success body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    /// Build a success response.
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build an error response.
    pub fn err(id: serde_json::Value, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(error),
        }
    }
}
