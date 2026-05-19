//! JSON-RPC error body. Codes follow the JSON-RPC 2.0 reserved
//! range; MCP-specific codes can be added as constants when needed.

use serde::Serialize;

/// JSON-RPC error frame.
#[derive(Debug, Serialize)]
pub struct RpcError {
    /// JSON-RPC error code. Negative values in the -32000…-32099
    /// range are reserved for server use.
    pub code: i32,

    /// Short human description.
    pub message: String,

    /// Optional structured detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    /// JSON-RPC -32601: method not found.
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("method not found: {method}"),
            data: None,
        }
    }

    /// JSON-RPC -32602: invalid params.
    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: detail.into(),
            data: None,
        }
    }

    /// JSON-RPC -32603: internal error.
    pub fn internal(detail: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: detail.into(),
            data: None,
        }
    }
}
