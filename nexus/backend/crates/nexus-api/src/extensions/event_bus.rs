//! In-process, tenant-scoped publish/subscribe bus for extensions (WS-18
//! Wave A).
//!
//! The bus is the cross-filter / live-update transport that replaces N
//! HTTP-loopback round-trips between extension surfaces. It is deliberately
//! small and self-contained:
//!
//! - **Topic-keyed.** Topics are reverse-DNS strings (`com.acme.charts.filter`).
//!   The publish-side namespace-ownership rule (an extension may publish only on
//!   topics it owns) is enforced at the host method, not here — this module is
//!   the transport.
//! - **Tenant-scoped.** A subscriber receives only messages published within its
//!   own tenant. The `(tenant, topic)` pair is the channel key, so cross-tenant
//!   leakage is structurally impossible — there is no code path that delivers a
//!   message across tenants.
//! - **At-most-once, drop-oldest.** Each `(tenant, topic)` is a
//!   [`tokio::sync::broadcast`] channel. A slow subscriber that falls
//!   `CHANNEL_CAPACITY` behind loses the oldest messages (the bus is a
//!   live-update transport where the latest value wins) rather than
//!   back-pressuring the publisher. Lag is surfaced to the subscriber as a
//!   skipped-count so it can resync if it cares.
//!
//! Wildcard subscription (`com.acme.charts.*`) matches the topic itself and any
//! dotted descendant; it is expanded at delivery time, not at channel-key time,
//! so a wildcard subscriber sees messages published on every matching concrete
//! topic.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use starter_ext_spi::event_bus::EventBusMessage;
use tokio::sync::broadcast;

/// Per-`(tenant, topic)` broadcast buffer depth. A subscriber more than this
/// many messages behind drops the oldest (broadcast `Lagged`).
const CHANNEL_CAPACITY: usize = 256;

/// A concrete-topic broadcast channel keyed by `(tenant_id, topic)`.
#[derive(Clone)]
struct Channel {
    sender: broadcast::Sender<EventBusMessage>,
}

/// In-process, tenant-scoped event bus shared across the control plane via
/// `AppState`. Cheap to clone (an `Arc` over the channel registry).
#[derive(Clone)]
pub struct ExtensionEventBus {
    inner: Arc<Inner>,
}

struct Inner {
    /// `tenant_id -> (topic -> channel)`. A `Mutex` (not `RwLock`) because the
    /// hot path (publish to an existing topic) only briefly locks to clone the
    /// sender, and subscribe/first-publish both mutate.
    channels: Mutex<HashMap<String, HashMap<String, Channel>>>,
}

impl Default for ExtensionEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionEventBus {
    /// Build an empty bus.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                channels: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Publish `payload` on `(tenant, topic)`, stamping a host `ts_unix_ms` so
    /// every subscriber sees the same timestamp regardless of fan-out path.
    /// Returns the number of subscribers the message was delivered to (0 when
    /// no one is listening — a publish to an empty topic is a no-op, not an
    /// error).
    ///
    /// Delivery reaches both exact-topic subscribers and wildcard subscribers
    /// whose pattern matches `topic`; both are modelled by also sending on the
    /// matching wildcard channels (see [`subscribe`](Self::subscribe)).
    pub fn publish(&self, tenant: &str, topic: &str, payload: serde_json::Value) -> usize {
        let msg = EventBusMessage {
            topic: topic.to_owned(),
            payload,
            ts_unix_ms: now_unix_ms(),
        };
        let targets = self.matching_senders(tenant, topic);
        let mut delivered = 0;
        for sender in targets {
            // `send` errors only when there are no receivers; that is a no-op
            // for us (the channel stays registered for a future subscriber).
            delivered += sender.send(msg.clone()).unwrap_or(0);
        }
        delivered
    }

    /// Subscribe to `(tenant, topic)`, returning a receiver the host fans
    /// matching messages into. `topic` may be a concrete topic
    /// (`com.acme.charts.filter`) or a single trailing-`*` wildcard
    /// (`com.acme.charts.*`), which matches the prefix topic and any dotted
    /// descendant.
    ///
    /// A wildcard subscription registers its own channel keyed by the literal
    /// pattern string; [`publish`](Self::publish) resolves every wildcard
    /// pattern a concrete topic matches and sends to those channels too, so the
    /// subscriber sees messages across all matching concrete topics without the
    /// bus having to know subscribers' patterns ahead of publish.
    pub fn subscribe(&self, tenant: &str, topic: &str) -> broadcast::Receiver<EventBusMessage> {
        let mut channels = self
            .inner
            .channels
            .lock()
            .expect("event bus mutex poisoned");
        let per_tenant = channels.entry(tenant.to_owned()).or_default();
        let channel = per_tenant.entry(topic.to_owned()).or_insert_with(|| {
            let (sender, _rx) = broadcast::channel(CHANNEL_CAPACITY);
            Channel { sender }
        });
        channel.sender.subscribe()
    }

    /// Drop a tenant's channels (called when the last context using them goes
    /// away is not tracked; this is a coarse cleanup hook for tenant teardown).
    /// Channels with no subscribers also self-prune lazily on the next publish
    /// that finds them empty.
    pub fn forget_tenant(&self, tenant: &str) {
        let mut channels = self
            .inner
            .channels
            .lock()
            .expect("event bus mutex poisoned");
        channels.remove(tenant);
    }

    /// Collect the senders a publish on `topic` must reach: the exact-topic
    /// channel plus every registered wildcard channel whose pattern matches.
    /// Prunes channels that have lost all receivers so the registry does not
    /// grow without bound.
    fn matching_senders(
        &self,
        tenant: &str,
        topic: &str,
    ) -> Vec<broadcast::Sender<EventBusMessage>> {
        let mut channels = self
            .inner
            .channels
            .lock()
            .expect("event bus mutex poisoned");
        let Some(per_tenant) = channels.get_mut(tenant) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        per_tenant.retain(|key, channel| {
            if channel.sender.receiver_count() == 0 {
                // No live subscribers — drop the channel (lazy prune). A future
                // subscriber re-creates it.
                return false;
            }
            if topic_matches(key, topic) {
                out.push(channel.sender.clone());
            }
            true
        });
        out
    }
}

/// `true` if `pattern` selects `topic`. `pattern` is either an exact topic or a
/// single trailing-`*` wildcard whose prefix matches `topic` exactly or as a
/// dotted ancestor (`com.acme.charts.*` matches `com.acme.charts` and
/// `com.acme.charts.filter`, but not `com.acme.chartsx`).
fn topic_matches(pattern: &str, topic: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix(".*") {
        topic == prefix
            || (topic.len() > prefix.len()
                && topic.starts_with(prefix)
                && topic.as_bytes()[prefix.len()] == b'.')
    } else if pattern == "*" {
        true
    } else {
        pattern == topic
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_matches_exact_and_wildcard() {
        assert!(topic_matches("com.a.b", "com.a.b"));
        assert!(!topic_matches("com.a.b", "com.a.c"));
        assert!(topic_matches("com.a.*", "com.a"));
        assert!(topic_matches("com.a.*", "com.a.b"));
        assert!(topic_matches("com.a.*", "com.a.b.c"));
        assert!(!topic_matches("com.a.*", "com.abc"));
        assert!(topic_matches("*", "anything.at.all"));
    }

    #[tokio::test]
    async fn exact_publish_reaches_subscriber() {
        let bus = ExtensionEventBus::new();
        let mut rx = bus.subscribe("t1", "com.acme.x");
        let n = bus.publish("t1", "com.acme.x", serde_json::json!({ "v": 1 }));
        assert_eq!(n, 1);
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.topic, "com.acme.x");
        assert_eq!(msg.payload, serde_json::json!({ "v": 1 }));
        assert!(msg.ts_unix_ms > 0);
    }

    #[tokio::test]
    async fn tenant_isolation_holds() {
        let bus = ExtensionEventBus::new();
        let mut rx_other = bus.subscribe("t2", "com.acme.x");
        // Publish in t1 — t2's subscriber must NOT receive it.
        let n = bus.publish("t1", "com.acme.x", serde_json::json!({ "v": 1 }));
        assert_eq!(n, 0, "no t1 subscriber, and t2 must not be reached");
        // Confirm nothing is queued for the other tenant.
        assert!(rx_other.try_recv().is_err());
    }

    #[tokio::test]
    async fn wildcard_subscriber_sees_child_topics() {
        let bus = ExtensionEventBus::new();
        let mut rx = bus.subscribe("t1", "com.acme.*");
        let n = bus.publish("t1", "com.acme.filter", serde_json::json!({ "k": "v" }));
        assert_eq!(n, 1);
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.topic, "com.acme.filter");
    }

    #[tokio::test]
    async fn empty_channels_are_pruned_on_publish() {
        let bus = ExtensionEventBus::new();
        {
            let _rx = bus.subscribe("t1", "com.acme.x");
            // _rx dropped at end of block -> receiver_count becomes 0.
        }
        // First publish after the receiver dropped prunes the channel and
        // reaches no one.
        let n = bus.publish("t1", "com.acme.x", serde_json::json!({}));
        assert_eq!(n, 0);
    }
}
