//! Wire types for the `tracing.event` host method.
//!
//! `ctx.tracing().event(level, msg, fields)` emits a structured
//! event the host can correlate with its own logging /
//! observability surface. For process-flavour extensions, the SDK
//! marshals each call as a JSON-RPC request (rather than a
//! notification) so the SDK's outbound channel stays a single
//! request/response shape — one round-trip per event is a
//! negligible overhead in v0.1 and keeps the SDK's `HostRpc`
//! demultiplexer simple.

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `tracing.event`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracingEventRequest {
    /// Lower-case level string (`"trace"`, `"debug"`, `"info"`,
    /// `"warn"`, `"error"`). Hosts map onto their own logging
    /// crate's level enum; unknown strings default to `"info"`.
    pub level: String,
    /// Human-readable message.
    pub msg: String,
    /// Optional structured fields. Hosts attach these to the
    /// log line however their backend supports (kv-pair fields,
    /// JSON sidecar, etc.).
    #[serde(default)]
    pub fields: serde_json::Value,
}

/// Wire response for `tracing.event`. Empty — every event
/// succeeds at the wire level; the host's logging backend
/// failing is not surfaced to the extension.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracingEventResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracing_event_round_trip() {
        let req = TracingEventRequest {
            level: "info".into(),
            msg: "hello".into(),
            fields: serde_json::json!({"k": "v"}),
        };
        let j = serde_json::to_value(&req).unwrap();
        let back: TracingEventRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);
    }
}
