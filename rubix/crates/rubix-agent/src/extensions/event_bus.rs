//! In-process publish-only event bus, plus its per-caller
//! [`EventBusBackend`] view.
//!
//! Storage is a `HashMap<topic, broadcast::Sender<EventBusMessage>>`
//! protected by a [`std::sync::Mutex`]. A topic's `Sender` is
//! created lazily on first publish or subscribe — the v1 SDK
//! handle does not surface `subscribe`, but the same channel is
//! used by the host's row-4 follow-up that adds the SDK-side
//! `subscribe(topic) -> BoxStream<EventBusMessage>` accessor.
//!
//! The host stamps `ts_unix_ms` before fan-out so every subscriber
//! observes the same timestamp regardless of fan-out latency.
//!
//! Per-caller namespace enforcement is **on the publish-side
//! backend**, not on the bus: an extension's
//! [`RubixEventBusBackend`] knows its own reverse-DNS namespace
//! (set by the per-call factory) and refuses topics that do not
//! start with it. A frame without a caller (system-internal) is
//! refused with [`starter_ext_spi::Error::Capability`] — the same
//! shape every tenant-scoped capability uses for the
//! "no-identity" refusal.

use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value as JsonValue;
use starter_ext_sdk::ctx::EventBusBackend;
use starter_ext_spi::event_bus::EventBusMessage;
use starter_ext_spi::Error;
use tokio::sync::broadcast;

/// Channel capacity for newly-created per-topic broadcasts.
///
/// Sized to soak short bursts (chart filter changes) without
/// pinning a slow subscriber. Slow subscribers see
/// `RecvError::Lagged` once the head laps them; this matches
/// `tokio::sync::broadcast` defaults across the workspace.
const TOPIC_CHANNEL_CAPACITY: usize = 256;

/// Process-local pub/sub bus shared by every
/// [`RubixEventBusBackend`] the [`super::backends::RubixCapabilityFactory`]
/// hands out.
///
/// Cheap to clone via `Arc`; one instance per agent. Tests
/// construct fresh instances per case (no global state).
#[derive(Debug, Default)]
pub struct RubixEventBus {
    topics: Mutex<HashMap<String, broadcast::Sender<EventBusMessage>>>,
}

impl RubixEventBus {
    /// Construct an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to `topic`. Creates the underlying channel on
    /// first call so publishers and subscribers can come up in any
    /// order. Returned receiver is the standard
    /// `tokio::sync::broadcast::Receiver` — callers handle
    /// `RecvError::Lagged` if they're slow.
    ///
    /// Used by the (future) SDK-side `subscribe(topic)` accessor
    /// and by unit tests that verify a publish fans out.
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<EventBusMessage> {
        let mut topics = self.topics.lock().expect("event-bus mutex poisoned");
        topics
            .entry(topic.to_owned())
            .or_insert_with(|| broadcast::channel(TOPIC_CHANNEL_CAPACITY).0)
            .subscribe()
    }

    /// Publish `payload` on `topic`. Lazily creates the channel
    /// when no subscriber has registered yet (so a publish that
    /// loses the race with a subscribe still allocates the same
    /// sender). Returns the number of currently-registered
    /// subscribers — `0` is a normal outcome when no consumer has
    /// connected yet, not an error.
    ///
    /// Wall-clock stamping uses `SystemTime::now()`; falls back to
    /// `0` if the clock is before the epoch (impossible in
    /// practice on supported platforms — same defensive default as
    /// the SDK's `WallClockBackend` stub).
    pub fn publish_raw(&self, topic: &str, payload: JsonValue) -> usize {
        let ts_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let msg = EventBusMessage {
            topic: topic.to_owned(),
            payload,
            ts_unix_ms,
        };
        let sender = {
            let mut topics = self.topics.lock().expect("event-bus mutex poisoned");
            topics
                .entry(topic.to_owned())
                .or_insert_with(|| broadcast::channel(TOPIC_CHANNEL_CAPACITY).0)
                .clone()
        };
        // `send` errors only when no receivers — which is fine,
        // the caller asked us to publish into the void. Map the
        // outcome to a subscriber count so observability code can
        // distinguish "fanned out" from "dropped on the floor".
        sender.send(msg).unwrap_or(0)
    }
}

/// Per-caller [`EventBusBackend`] handed to a single extension's
/// `CtxInner`.
///
/// `caller_namespace` is the calling extension's reverse-DNS id
/// (e.g. `"com.acme.charts"`); the per-call factory populates it
/// when constructing the handle. A backend with
/// `caller_namespace = None` represents a host-internal / system
/// frame and refuses every publish — same shape as the warehouse
/// backend's caller-tenant refusal.
#[derive(Debug, Clone)]
pub struct RubixEventBusBackend {
    bus: std::sync::Arc<RubixEventBus>,
    caller_namespace: Option<String>,
    /// Topics the manifest's `event_bus.publish` grant permits.
    /// `None` ⇒ host-internal frame (per-topic gate skipped — the
    /// namespace check still fires); `Some(set)` ⇒ allowlist (the
    /// empty set is the explicit neutralised grant — every publish
    /// refused).
    granted_publish_topics: Option<BTreeSet<String>>,
}

impl RubixEventBusBackend {
    /// Construct a backend bound to `bus` and `caller_namespace`,
    /// with no per-topic manifest gate.
    ///
    /// Prefer [`Self::with_grant`] from any flow that has a sealed
    /// `ExtensionRegistry` — the grant is the manifest-side
    /// allowlist that runs *before* the namespace check, so an
    /// operator-neutralised grant refuses every publish even on
    /// in-namespace topics.
    pub fn new(bus: std::sync::Arc<RubixEventBus>, caller_namespace: Option<String>) -> Self {
        Self {
            bus,
            caller_namespace,
            granted_publish_topics: None,
        }
    }

    /// Construct with both the caller namespace and the resolved
    /// manifest grant. Pass `None` for `granted_publish_topics`
    /// to keep the host-internal posture (gate skipped).
    pub fn with_grant(
        bus: std::sync::Arc<RubixEventBus>,
        caller_namespace: Option<String>,
        granted_publish_topics: Option<BTreeSet<String>>,
    ) -> Self {
        Self {
            bus,
            caller_namespace,
            granted_publish_topics,
        }
    }

    fn check_publish_grant(&self, topic: &str) -> Result<(), Error> {
        if let Some(grant) = &self.granted_publish_topics {
            if !grant.contains(topic) {
                return Err(Error::capability(format!(
                    "event_bus.publish {topic:?} refused: not in calling extension's \
                     `event_bus.publish` grant"
                )));
            }
        }
        Ok(())
    }

    /// `true` if `topic` is owned by `caller_namespace`.
    ///
    /// Ownership is exact-namespace-prefix with a trailing dot
    /// (so `"com.acme.charts"` may publish on
    /// `"com.acme.charts.filter"` but not on `"com.acme.chartszzz"`
    /// or `"com.evil"`). A topic equal to the namespace itself is
    /// also permitted (the extension is publishing on its own root
    /// stream).
    fn topic_is_owned(namespace: &str, topic: &str) -> bool {
        if topic == namespace {
            return true;
        }
        let mut owned_prefix = String::with_capacity(namespace.len() + 1);
        owned_prefix.push_str(namespace);
        owned_prefix.push('.');
        topic.starts_with(&owned_prefix)
    }
}

impl EventBusBackend for RubixEventBusBackend {
    fn publish(&self, topic: &str, payload: JsonValue) -> Result<(), Error> {
        // Per-topic manifest grant first: an operator-neutralised
        // grant refuses every publish, including on in-namespace
        // topics. The namespace check is the second-layer kernel
        // invariant (the supervisor can't trust a misconfigured
        // host to set the grant correctly, but it *can* refuse
        // cross-namespace publishes regardless).
        self.check_publish_grant(topic)?;
        let Some(namespace) = self.caller_namespace.as_deref() else {
            return Err(Error::capability(
                "event_bus.publish refused: no caller identity (system frame)",
            ));
        };
        if !Self::topic_is_owned(namespace, topic) {
            return Err(Error::capability(format!(
                "event_bus.publish refused: topic {topic:?} is outside the calling extension's namespace {namespace:?}"
            )));
        }
        self.bus.publish_raw(topic, payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn topic_ownership_rules() {
        let ns = "com.acme.charts";
        assert!(RubixEventBusBackend::topic_is_owned(ns, ns));
        assert!(RubixEventBusBackend::topic_is_owned(
            ns,
            "com.acme.charts.filter"
        ));
        assert!(RubixEventBusBackend::topic_is_owned(
            ns,
            "com.acme.charts.filter.site"
        ));
        // Same prefix but no dot boundary — not a sub-topic.
        assert!(!RubixEventBusBackend::topic_is_owned(
            ns,
            "com.acme.chartszzz"
        ));
        assert!(!RubixEventBusBackend::topic_is_owned(ns, "com.evil"));
        assert!(!RubixEventBusBackend::topic_is_owned(ns, "com.acme.other"));
    }

    #[test]
    fn publish_without_caller_is_refused() {
        let bus = Arc::new(RubixEventBus::new());
        let backend = RubixEventBusBackend::new(bus, None);
        let err = backend
            .publish("any.topic", JsonValue::Null)
            .expect_err("system frame must be refused");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[test]
    fn publish_outside_namespace_is_refused() {
        let bus = Arc::new(RubixEventBus::new());
        let backend = RubixEventBusBackend::new(bus, Some("com.acme.charts".to_owned()));
        let err = backend
            .publish("com.evil.steal", JsonValue::Null)
            .expect_err("cross-namespace publish must be refused");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn publish_fans_out_with_timestamp_stamped() {
        let bus = Arc::new(RubixEventBus::new());
        let mut rx = bus.subscribe("com.acme.charts.filter");
        let backend = RubixEventBusBackend::new(bus.clone(), Some("com.acme.charts".to_owned()));

        backend
            .publish("com.acme.charts.filter", serde_json::json!({"site": "s1"}))
            .expect("publish ok");

        let msg = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
            .await
            .expect("recv timed out")
            .expect("recv failed");
        assert_eq!(msg.topic, "com.acme.charts.filter");
        assert_eq!(msg.payload, serde_json::json!({"site": "s1"}));
        assert!(msg.ts_unix_ms > 0, "host must stamp ts_unix_ms");
    }

    #[test]
    fn publish_outside_grant_is_refused_even_in_namespace() {
        let bus = Arc::new(RubixEventBus::new());
        // Manifest granted publish on `com.acme.charts.filter` only —
        // a sibling topic in the same namespace must still refuse.
        let grant: BTreeSet<String> = ["com.acme.charts.filter".to_string()].into_iter().collect();
        let backend =
            RubixEventBusBackend::with_grant(bus, Some("com.acme.charts".to_owned()), Some(grant));
        let err = backend
            .publish("com.acme.charts.other", JsonValue::Null)
            .expect_err("out-of-grant topic must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[test]
    fn publish_inside_grant_passes_to_namespace_check() {
        let bus = Arc::new(RubixEventBus::new());
        let grant: BTreeSet<String> = ["com.acme.charts.filter".to_string()].into_iter().collect();
        let backend =
            RubixEventBusBackend::with_grant(bus, Some("com.acme.charts".to_owned()), Some(grant));
        backend
            .publish("com.acme.charts.filter", JsonValue::Null)
            .expect("in-grant + in-namespace publishes");
    }

    #[test]
    fn neutralised_grant_refuses_every_publish() {
        let bus = Arc::new(RubixEventBus::new());
        // Operator-neutralised grant: empty allowlist refuses
        // every publish even on in-namespace topics. This is the
        // same shape every other Row-5 grant uses.
        let backend = RubixEventBusBackend::with_grant(
            bus,
            Some("com.acme.charts".to_owned()),
            Some(BTreeSet::new()),
        );
        let err = backend
            .publish("com.acme.charts.filter", JsonValue::Null)
            .expect_err("neutralised grant must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[test]
    fn grant_runs_before_namespace_check() {
        // No namespace set + a permissive grant should still refuse
        // because the per-topic gate fires first only when set —
        // but the namespace check kicks in second. Conversely, when
        // a non-empty grant *does* contain a cross-namespace topic
        // (an over-broad operator grant), the kernel-side namespace
        // check still refuses it. The two layers compose: the
        // *tighter* one wins.
        let bus = Arc::new(RubixEventBus::new());
        let grant: BTreeSet<String> = ["com.evil.cross".to_string()].into_iter().collect();
        let backend =
            RubixEventBusBackend::with_grant(bus, Some("com.acme.charts".to_owned()), Some(grant));
        let err = backend
            .publish("com.evil.cross", JsonValue::Null)
            .expect_err("namespace gate still refuses an over-broad operator grant");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[test]
    fn publish_with_no_subscribers_is_ok() {
        let bus = Arc::new(RubixEventBus::new());
        let backend = RubixEventBusBackend::new(bus, Some("com.acme.charts".to_owned()));
        // No subscriber registered yet — publish is still Ok.
        backend
            .publish("com.acme.charts.filter", JsonValue::Null)
            .expect("publish-into-void is ok");
    }
}
