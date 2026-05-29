//! Shared SSE wire shape for streaming HTTP responses.
//!
//! Extracted from [`super::chat_stream`] so both the chat surface
//! (`POST /api/v1/chat/stream`) and the admin streaming invoke
//! surface (`POST /api/v1/admin/registry/tools/{id}/invoke/stream`)
//! emit *one* frame shape that a single client decoder can consume.
//!
//! All non-payload fields are `Option` with `skip_serializing_if`
//! so each surface emits only the keys it cares about — clients
//! switch on `type` and pluck what they need. New variants and
//! fields are additive. See
//! [docs/design/admin/](../../../../docs/design/admin/README.md)
//! §"Streaming invoke".

use std::convert::Infallible;

use axum::response::sse::Event as SseEvent;
use serde::Serialize;
use serde_json::{json, Value};
use tracing::warn;

/// One frame on the SSE wire.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamFrame {
    Connected {
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    Text {
        delta: String,
    },
    ToolUse {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<Value>,
    },
    Result {
        value: Value,
    },
    Done {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cost_usd: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        latency_ms: Option<u64>,
    },
    Error {
        message: String,
    },
}

impl StreamFrame {
    /// Chat `done` frame — token + cost accounting.
    pub fn done_chat(
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
        duration_ms: u64,
    ) -> Self {
        StreamFrame::Done {
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            cost_usd: Some(cost_usd),
            duration_ms: Some(duration_ms),
            status: None,
            latency_ms: None,
        }
    }

    /// Admin streaming invoke `done` frame — invocation status +
    /// latency.
    pub fn done_invoke(status: impl Into<String>, latency_ms: u64) -> Self {
        StreamFrame::Done {
            input_tokens: None,
            output_tokens: None,
            cost_usd: None,
            duration_ms: None,
            status: Some(status.into()),
            latency_ms: Some(latency_ms),
        }
    }
}

/// Serialise a frame as an SSE `data:` event.
pub fn frame_to_sse(frame: &StreamFrame) -> Result<SseEvent, Infallible> {
    match serde_json::to_string(frame) {
        Ok(s) => Ok(SseEvent::default().data(s)),
        Err(e) => {
            warn!(
                target: "rubix.routes.stream_frames",
                error = %e,
                "stream frame serialisation failed",
            );
            Ok(SseEvent::default()
                .data(json!({"type":"error","message":"frame serialisation failed"}).to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_done_serialises_the_chat_keys() {
        let f = StreamFrame::done_chat(10, 20, 0.001, 100);
        let v: Value = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(v["type"], "done");
        assert_eq!(v["input_tokens"], 10);
        assert_eq!(v["output_tokens"], 20);
        assert_eq!(v["duration_ms"], 100);
        assert!(v.get("status").is_none());
        assert!(v.get("latency_ms").is_none());
    }

    #[test]
    fn invoke_done_serialises_the_invoke_keys() {
        let f = StreamFrame::done_invoke("ok", 12);
        let v: Value = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(v["type"], "done");
        assert_eq!(v["status"], "ok");
        assert_eq!(v["latency_ms"], 12);
        assert!(v.get("input_tokens").is_none());
    }

    #[test]
    fn result_frame_carries_payload() {
        let f = StreamFrame::Result {
            value: json!({"ok": true}),
        };
        let v: Value = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(v["type"], "result");
        assert_eq!(v["value"], json!({"ok": true}));
    }
}
