//! [`Event`] — the (kind, payload) pair every `EventSink::emit` is
//! handed.
//!
//! The starter SPI does not opine on the consumer's event type. The
//! blanket [`EventSink`](super::EventSink) impl on
//! `tokio::sync::broadcast::Sender<E>` requires `E: From<Event>` so a
//! consumer can fan their domain `Event` enum through the same sink
//! without writing glue.

use serde_json::Value;

/// A minimal event envelope.
///
/// `kind` is the stable service-supplied string (`"slack.message"`,
/// `"telegram.update"`, …). `payload` is the deserialized provider
/// event as JSON. Consumers typically pattern-match `kind` and
/// re-deserialize `payload` into a typed struct.
#[derive(Debug, Clone)]
pub struct Event {
    /// Stable event-kind string supplied by the service.
    pub kind: String,
    /// Provider event body as JSON.
    pub payload: Value,
}

impl Event {
    /// Construct an event from owned parts.
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
        }
    }
}
