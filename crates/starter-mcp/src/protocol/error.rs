//! JSON-RPC error body. Codes follow the JSON-RPC 2.0 reserved
//! range; MCP-specific codes can be added as constants when needed.

use serde::Serialize;
use starter_spi::error::Error as SpiError;

/// JSON-RPC error code for `Error::NotFound`. Picked from the
/// server-reserved -32000…-32099 range; documented here so clients
/// can pattern-match.
pub const CODE_NOT_FOUND: i32 = -32004;
/// JSON-RPC error code for `Error::Conflict`.
pub const CODE_CONFLICT: i32 = -32009;
/// JSON-RPC error code for `Error::Unauthenticated`.
pub const CODE_UNAUTHENTICATED: i32 = -32001;
/// JSON-RPC error code for `Error::Forbidden`.
pub const CODE_FORBIDDEN: i32 = -32002;

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

    /// Map a `starter_spi::Error` onto a JSON-RPC error frame.
    ///
    /// Preserves the domain category in `code` so clients can
    /// pattern-match without parsing strings:
    ///
    /// | Variant            | Code     |
    /// |--------------------|----------|
    /// | `Invalid`          | -32602   |
    /// | `NotFound`         | -32004   |
    /// | `Conflict`         | -32009   |
    /// | `Unauthenticated`  | -32001   |
    /// | `Forbidden`        | -32002   |
    /// | `Internal`         | -32603   |
    ///
    /// Tool implementations that build messages with a leading
    /// diagnostic code (e.g. `"rubix.dashboard.update.conflict:
    /// page_id=..."`) get that code lifted into `data.code` so it
    /// stays stable on the wire even if the human message is
    /// localised or rephrased later.
    pub fn from_spi(err: &SpiError) -> Self {
        // `Error` is `#[non_exhaustive]`. The internal / unknown
        // branch handles its own data shape (source chain) so it's
        // extracted up front; the named-variant branches share the
        // diagnostic-code-lifting tail below.
        if matches!(err, SpiError::Internal { .. }) {
            return Self::internal_from_source(err);
        }
        let (code, message) = match err {
            SpiError::Invalid { message } => (-32602, message.clone()),
            SpiError::NotFound { what } => (CODE_NOT_FOUND, format!("not found: {what}")),
            SpiError::Conflict { message } => (CODE_CONFLICT, message.clone()),
            SpiError::Unauthenticated => (CODE_UNAUTHENTICATED, "unauthenticated".to_owned()),
            SpiError::Forbidden => (CODE_FORBIDDEN, "forbidden".to_owned()),
            // Future variants added under `#[non_exhaustive]` —
            // fall back to internal-error semantics with the chain
            // so the wire still carries something diagnosable.
            _ => return Self::internal_from_source(err),
        };

        // Lift a leading `domain.dotted.diagnostic: ...` prefix into
        // `data.code` so clients can branch on the diagnostic without
        // string-matching the message. The grammar mirrors what
        // `starter_spi::i18n::Diagnostic` produces.
        let diagnostic_code = leading_diagnostic_code(&message);
        let mut data_obj = serde_json::Map::new();
        if let Some(dc) = diagnostic_code {
            data_obj.insert("code".to_owned(), serde_json::Value::String(dc.to_owned()));
        }
        let data = if data_obj.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(data_obj))
        };

        Self {
            code,
            message,
            data,
        }
    }
}

/// Extract a leading `domain.dotted.code` prefix from a message of
/// the form `"<code>: <detail>"`. Returns `None` when the prefix is
/// missing, doesn't contain a dot, or contains characters outside
/// the diagnostic-key grammar.
fn leading_diagnostic_code(message: &str) -> Option<&str> {
    let (head, _) = message.split_once(": ")?;
    if !head.contains('.') {
        return None;
    }
    if !head
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return None;
    }
    Some(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_spi_maps_invalid_to_minus_32602() {
        let err = SpiError::Invalid {
            message: "page_id missing".to_owned(),
        };
        let rpc = RpcError::from_spi(&err);
        assert_eq!(rpc.code, -32602);
        assert_eq!(rpc.message, "page_id missing");
        assert!(rpc.data.is_none(), "no diagnostic code → no `data`");
    }

    #[test]
    fn from_spi_maps_not_found_to_custom_code() {
        let err = SpiError::NotFound {
            what: "dashboard `system:dashboard.x`".to_owned(),
        };
        let rpc = RpcError::from_spi(&err);
        assert_eq!(rpc.code, CODE_NOT_FOUND);
        assert!(rpc.message.starts_with("not found:"));
    }

    #[test]
    fn from_spi_lifts_diagnostic_code_into_data() {
        let err = SpiError::Conflict {
            message: "rubix.dashboard.update.conflict: page_id=ops".to_owned(),
        };
        let rpc = RpcError::from_spi(&err);
        assert_eq!(rpc.code, CODE_CONFLICT);
        assert_eq!(
            rpc.data.as_ref().unwrap()["code"],
            "rubix.dashboard.update.conflict",
        );
    }

    #[test]
    fn from_spi_internal_preserves_chain() {
        let inner: Box<dyn std::error::Error + Send + Sync> = "db gone".into();
        let err = SpiError::Internal { source: inner };
        let rpc = RpcError::from_spi(&err);
        assert_eq!(rpc.code, -32603);
        assert!(rpc.data.as_ref().unwrap()["chain"].is_array());
    }

    #[test]
    fn leading_diagnostic_code_rejects_non_dotted_prefix() {
        assert_eq!(leading_diagnostic_code("conflict: x"), None);
        assert_eq!(leading_diagnostic_code("no prefix here"), None);
        assert_eq!(
            leading_diagnostic_code("rubix.dashboard.created: ok"),
            Some("rubix.dashboard.created"),
        );
    }
}
