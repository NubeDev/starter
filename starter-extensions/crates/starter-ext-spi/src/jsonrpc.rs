//! JSON-RPC 2.0 wire envelope shared by every adapter.
//!
//! Per SCOPE.md **R10**: process-flavour extensions speak JSON-RPC 2.0 with
//! `Content-Length` framing over stdio. The framing itself is implemented in
//! the `starter-jsonrpc-stdio` crate (in the parent workspace, consumed by
//! both `starter-mcp` and `starter-ext-supervisor`). This module is the
//! *type* layer — the `JsonRpcEnvelope` enum plus the streaming
//! sub-protocol — so every adapter (`mcp`, `rest`, `cli`, `grpc`,
//! `workers`, `ui`) marshals through the same shape.
//!
//! ## Streaming sub-protocol (SCOPE post-R13)
//!
//! Long-running calls return one initial response carrying a `stream_id`
//! and then emit notifications with reserved method names:
//!
//! - `stream.event` — one payload chunk for an open stream.
//! - `stream.end`   — normal termination (no more events).
//! - `stream.error` — abnormal termination (payload is an [`Error`]).
//! - `stream.cancel` — cancellation; host→extension *or* extension→host.
//!
//! Adapters translate this shape into their transport's native conventions
//! (SSE frames, gRPC server-streaming, MCP notifications, …); the *kernel*
//! shape is one.
//!
//! [`Error`]: crate::Error

use serde::{Deserialize, Serialize};

use crate::Error;

/// The JSON-RPC version every envelope carries.
pub const JSONRPC_VERSION: &str = "2.0";

/// Reserved method names for the streaming sub-protocol.
///
/// Adapter code matches against these constants rather than re-typing the
/// strings — kept here so a typo can never silently invent a new method.
pub mod stream_methods {
    /// One payload chunk for an open stream.
    pub const EVENT: &str = "stream.event";
    /// Normal termination.
    pub const END: &str = "stream.end";
    /// Abnormal termination.
    pub const ERROR: &str = "stream.error";
    /// Cancellation (either direction).
    pub const CANCEL: &str = "stream.cancel";
}

/// The one new wire method the FLOW-NODES track contributes
/// (`DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-1). The host's
/// [`crate::manifest::ContributeNode`] declares a node kind; the
/// child process implements the body and the host invokes it through
/// this method.
///
/// Adapters that need to recognise the method on the wire match
/// against this constant rather than re-typing the string so a typo
/// can never silently invent a parallel method.
pub const FLOW_NODE_INVOKE: &str = "flow.node.invoke";

/// JSON-RPC error codes reserved for the flow-node dispatch path
/// (`DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-1 +
/// R-flow-node-5).
///
/// The JSON-RPC 2.0 spec reserves `-32768..-32000` for protocol-level
/// errors. The host's flow-node dispatch carves out the
/// `-32050..-32099` window from the application-level range so a
/// child can report node-specific failures without colliding with
/// other adapters' codes (MCP carves out a different window, gRPC
/// uses its own status enum).
///
/// Codes are non-overlapping within the window; adding a code is
/// additive within the crate major. Adapters that need to recognise
/// a flow-node error map this enum against the inbound envelope's
/// `error.code` field; everything outside the window stays opaque so
/// the kernel never claims ownership of a code another extension
/// minted.
pub mod flow_node_error_codes {
    /// Bottom of the flow-node carve-out (inclusive). Adapters
    /// validating an inbound code use `code >= RANGE_START` as the
    /// first half of the window check.
    pub const RANGE_START: i32 = -32099;

    /// Top of the flow-node carve-out (inclusive). Adapters
    /// validating an inbound code use `code <= RANGE_END` as the
    /// second half of the window check.
    pub const RANGE_END: i32 = -32050;

    /// `flow.node.invoke` was issued against a `kind` the child does
    /// not implement (or one no longer present after a hot-reload).
    /// The proxy surfaces this as
    /// [`crate::Error::ExtensionInternal`] for now; richer mapping
    /// lands when the engine grows per-code routing.
    pub const NODE_KIND_NOT_BOUND: i32 = -32050;

    /// The child failed to parse `flow.node.invoke` params (missing
    /// `invocation_id`, malformed `settings`, …). Distinct from a
    /// JSON-RPC `Invalid Params` (-32602) so the host can tell
    /// "child's parser disagrees with the manifest" apart from
    /// "child crashed parsing JSON".
    pub const INVALID_INVOCATION_PARAMS: i32 = -32051;

    /// The child's body returned a typed `NodeError::Backend`-shaped
    /// failure (broker rejected, downstream HTTP 5xx, …). The proxy
    /// maps this back to
    /// [`starter_flow_spi::node::NodeError::Backend`] without
    /// disturbing the surrounding stream notifications.
    pub const NODE_BACKEND: i32 = -32060;

    /// The child observed its own `stream.cancel` and is shutting
    /// the invocation down cleanly. The proxy maps this back to
    /// [`starter_flow_spi::node::NodeError::Cancelled`] so the
    /// engine emits `NodeCancelled`.
    pub const NODE_CANCELLED: i32 = -32061;

    /// Returns `true` if `code` falls inside the flow-node carve-out
    /// window (inclusive both ends).
    #[inline]
    pub const fn is_in_range(code: i32) -> bool {
        code >= RANGE_START && code <= RANGE_END
    }
}

/// An opaque stream identifier returned by the initial request and echoed
/// on every subsequent notification belonging to that stream.
///
/// Newtype over `String` so adapters cannot accidentally pass a request id
/// where a stream id is expected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StreamId(pub String);

impl StreamId {
    /// Borrow the inner string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A JSON-RPC 2.0 request id. JSON-RPC permits string, number, or null;
/// `null` is reserved for notifications elsewhere so we accept the two
/// useful shapes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// Numeric id — what most clients emit.
    Number(i64),
    /// String id — useful for correlation when multiple senders share a
    /// transport.
    String(String),
}

/// A JSON-RPC 2.0 envelope: request, response, or notification.
///
/// `#[serde(untagged)]` so the wire form is the bare JSON-RPC document
/// without an extra discriminator. Deserialisation picks the right variant
/// from the presence of `method`, `id`, `result`, and `error` per the
/// specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcEnvelope {
    /// A request (has `id` and `method`).
    Request(JsonRpcRequest),
    /// A response (has `id` and either `result` or `error`).
    Response(JsonRpcResponse),
    /// A notification (has `method`, no `id`).
    Notification(JsonRpcNotification),
}

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// The literal `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id; the matching response echoes this value.
    pub id: JsonRpcId,
    /// Method name. For extension dispatch this is a manifest-declared
    /// contribute id (tool id, route id, …).
    pub method: String,
    /// Method parameters; opaque JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// The literal `"2.0"`.
    pub jsonrpc: String,
    /// Echoes the request's id.
    pub id: JsonRpcId,
    /// Either a successful result or a transport-level error.
    #[serde(flatten)]
    pub payload: JsonRpcResponsePayload,
}

/// The success / failure payload of a response.
///
/// JSON-RPC 2.0 requires exactly one of `result` or `error`. Modelling it
/// as a flattened enum makes the type system enforce that.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonRpcResponsePayload {
    /// Success.
    Result(serde_json::Value),
    /// Transport-level failure. Application errors raised by an extension
    /// handler are wrapped in [`Error::ExtensionInternal`] and surface as
    /// the `error` payload.
    Error(Error),
}

/// A JSON-RPC 2.0 notification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// The literal `"2.0"`.
    pub jsonrpc: String,
    /// Method name. For streaming, one of `stream_methods::*`.
    pub method: String,
    /// Method parameters; opaque JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Streaming sub-protocol payloads
// ---------------------------------------------------------------------------

/// Typed view of a `stream.event` notification's params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamEvent {
    /// The stream this chunk belongs to.
    pub stream_id: StreamId,
    /// Opaque payload. Adapters interpret per-contribute-type.
    pub payload: serde_json::Value,
}

/// Typed view of a `stream.end` notification's params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamEnd {
    /// The stream that just terminated normally.
    pub stream_id: StreamId,
    /// Optional final payload (e.g. summary, totals). Adapters that have
    /// no use for it ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// Typed view of a `stream.error` notification's params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamError {
    /// The stream that terminated abnormally.
    pub stream_id: StreamId,
    /// Reason. Maps onto adapter-native error shapes.
    pub error: Error,
}

/// Typed view of a `stream.cancel` notification's params.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamCancel {
    /// The stream the sender wants cancelled.
    pub stream_id: StreamId,
    /// Optional human-readable reason for logging.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A typed view of a streaming notification.
///
/// Helper type for adapter code that wants pattern-matching instead of
/// string-matching on `method`. Constructed by inspecting a
/// [`JsonRpcNotification`] whose method is one of `stream_methods::*`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum StreamNotification {
    /// `stream.event`
    #[serde(rename = "stream.event")]
    Event(StreamEvent),
    /// `stream.end`
    #[serde(rename = "stream.end")]
    End(StreamEnd),
    /// `stream.error`
    #[serde(rename = "stream.error")]
    Error(StreamError),
    /// `stream.cancel`
    #[serde(rename = "stream.cancel")]
    Cancel(StreamCancel),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trip() {
        let req = JsonRpcRequest {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: JsonRpcId::Number(1),
            method: "com.acme.weather.current".to_string(),
            params: Some(serde_json::json!({ "city": "Sydney" })),
        };
        let env = JsonRpcEnvelope::Request(req.clone());
        let j = serde_json::to_string(&env).unwrap();
        let back: JsonRpcEnvelope = serde_json::from_str(&j).unwrap();
        match back {
            JsonRpcEnvelope::Request(r) => assert_eq!(r, req),
            other => panic!("expected request, got {:?}", other),
        }
    }

    #[test]
    fn response_success_and_error() {
        let ok = JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: JsonRpcId::Number(1),
            payload: JsonRpcResponsePayload::Result(serde_json::json!({ "temp_c": 22 })),
        };
        let j = serde_json::to_string(&ok).unwrap();
        assert!(j.contains("\"result\""));

        let err = JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: JsonRpcId::Number(2),
            payload: JsonRpcResponsePayload::Error(Error::Capability("no http_out".into())),
        };
        let j = serde_json::to_string(&err).unwrap();
        assert!(j.contains("\"error\""));
    }

    #[test]
    fn stream_notification_round_trip() {
        let n = StreamNotification::Event(StreamEvent {
            stream_id: StreamId("s-1".into()),
            payload: serde_json::json!({ "line": "hello" }),
        });
        let j = serde_json::to_string(&n).unwrap();
        let back: StreamNotification = serde_json::from_str(&j).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn flow_node_invoke_method_is_stable() {
        // If somebody renames this the wire shape moves with it — guard
        // the value so the typo never escapes review.
        assert_eq!(FLOW_NODE_INVOKE, "flow.node.invoke");
    }

    #[test]
    fn flow_node_error_codes_inside_carve_out() {
        use flow_node_error_codes::*;
        assert!(is_in_range(NODE_KIND_NOT_BOUND));
        assert!(is_in_range(INVALID_INVOCATION_PARAMS));
        assert!(is_in_range(NODE_BACKEND));
        assert!(is_in_range(NODE_CANCELLED));
        assert!(is_in_range(RANGE_START));
        assert!(is_in_range(RANGE_END));
        // The window stops at -32050: anything one step warmer is
        // outside the carve-out so other adapters can claim it.
        assert!(!is_in_range(-32049));
        assert!(!is_in_range(-32100));
        // JSON-RPC's `Invalid Params` is intentionally outside.
        assert!(!is_in_range(-32602));
    }

    #[test]
    fn stream_method_constants_match() {
        // If somebody renames a constant, the wire shape moves with it —
        // this test makes that drift visible.
        assert_eq!(stream_methods::EVENT, "stream.event");
        assert_eq!(stream_methods::END, "stream.end");
        assert_eq!(stream_methods::ERROR, "stream.error");
        assert_eq!(stream_methods::CANCEL, "stream.cancel");
    }

    #[test]
    fn envelope_dispatches_on_shape() {
        // Notification: no `id`, has `method`.
        let raw = r#"{"jsonrpc":"2.0","method":"stream.end","params":{"stream_id":"s-1"}}"#;
        let env: JsonRpcEnvelope = serde_json::from_str(raw).unwrap();
        assert!(matches!(env, JsonRpcEnvelope::Notification(_)));

        // Response: has `id` and `result`.
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"stream_id":"s-1"}}"#;
        let env: JsonRpcEnvelope = serde_json::from_str(raw).unwrap();
        assert!(matches!(env, JsonRpcEnvelope::Response(_)));

        // Request: has `id` and `method`.
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"foo.bar"}"#;
        let env: JsonRpcEnvelope = serde_json::from_str(raw).unwrap();
        assert!(matches!(env, JsonRpcEnvelope::Request(_)));
    }
}
