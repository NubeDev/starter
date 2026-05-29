//! Wire types for the `event_bus` capability (publish/subscribe).
//!
//! Per [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
//! row 4. The bus is the cross-filter / live-update transport
//! that replaces N-per-click HTTP loopback round-trips in a
//! Power-BI-style dashboard surface.
//!
//! `starter-ext-spi` is contract-only (R2: no I/O). The handle,
//! supervisor namespace, and host-side fan-out live in
//! `starter-ext-sdk`, `starter-ext-supervisor`, and the host
//! integration crate respectively.

use serde::{Deserialize, Serialize};

/// One message on the event bus.
///
/// Carried verbatim through the bus — the host does not inspect
/// `payload`. Topic strings are reverse-DNS (`com.acme.charts.filter`);
/// the supervisor enforces publish-side namespace ownership the same
/// way it enforces tool ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBusMessage {
    /// Reverse-DNS topic the message was published on.
    pub topic: String,
    /// Opaque payload. Schema is documented per-topic by the
    /// publishing extension; the bus does not validate it.
    pub payload: serde_json::Value,
    /// Host-stamped publish timestamp (Unix epoch ms). Set by the
    /// host on `publish` so subscribers see the same value
    /// regardless of fan-out path.
    pub ts_unix_ms: u64,
}

/// Wire request for the `event_bus.publish` host method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBusPublishRequest {
    /// Topic to publish on. Must be in the extension's grant
    /// `publish: […]` allowlist; the supervisor rejects calls on
    /// disallowed topics with `Error::Capability`.
    pub topic: String,
    /// Payload to broadcast. Forwarded to subscribers verbatim.
    pub payload: serde_json::Value,
}

/// Wire response for `event_bus.publish`. Empty struct so future
/// fields (e.g. subscriber count, broadcast timestamp) can land
/// additively without breaking the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBusPublishResponse {}

/// Wire request for the `event_bus.subscribe` host method (row-4
/// follow-up — the v1 handle only exposes `publish`).
///
/// Captured in the SPI now so the wire shape doesn't break when the
/// subscribe-side handle lands: a follow-up adds the
/// `EventBusHandle::subscribe(topic)` accessor returning a
/// `BoxStream<EventBusMessage>` over the existing `stream.event` /
/// `stream.end` / `stream.cancel` notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBusSubscribeRequest {
    /// Topic to subscribe to. Must be in the extension's grant
    /// `subscribe: […]` allowlist.
    pub topic: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_round_trip() {
        let m = EventBusMessage {
            topic: "com.acme.charts.filter".into(),
            payload: serde_json::json!({ "site_id": "s-1" }),
            ts_unix_ms: 1_700_000_000_000,
        };
        let j = serde_json::to_string(&m).unwrap();
        let back: EventBusMessage = serde_json::from_str(&j).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn publish_request_round_trip() {
        let r = EventBusPublishRequest {
            topic: "com.acme.charts.filter".into(),
            payload: serde_json::json!({}),
        };
        let back: EventBusPublishRequest =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn subscribe_request_round_trip() {
        let r = EventBusSubscribeRequest {
            topic: "com.acme.charts.*".into(),
        };
        let back: EventBusSubscribeRequest =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
