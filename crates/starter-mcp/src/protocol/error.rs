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

    /// JSON-RPC -32603 built from a `std::error::Error`. Walks the
    /// `source()` chain so the wire payload carries every link, not
    /// just the outermost `Display`. The full chain is rendered as a
    /// `"<outer>: <inner>: <innermost>"` summary in `message` and
    /// also serialised into `data.chain` (a JSON array) so a client
    /// can render the layers individually.
    ///
    /// Several core errors in the workspace
    /// (`starter_spi::Error::Internal`, `sqlx::Error`, others) carry
    /// a generic outer `Display` and put the real cause in the
    /// `source()`. The plain [`Self::internal`] constructor collapses
    /// to the outer string and loses the cause; this constructor is
    /// the recommended way to map any boxed error into a wire frame.
    pub fn internal_from_source(err: &(dyn std::error::Error + 'static)) -> Self {
        let mut chain: Vec<String> = Vec::new();
        chain.push(err.to_string());
        let mut current = err.source();
        while let Some(next) = current {
            chain.push(next.to_string());
            current = next.source();
        }
        let message = chain.join(": ");
        let data = serde_json::json!({ "chain": chain });
        Self {
            code: -32603,
            message,
            data: Some(data),
        }
    }
}
