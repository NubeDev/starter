//! [`EventSink`] — where a [`Service`](super::Service) publishes its
//! events. The single point of contact between provider code and
//! consumer code (SCOPE R4).

use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// Typed sink failures. See Decision D4 in `DOCS/tools/scope/SCOPE.md`.
///
/// The fan-out helper logs-and-continues on `Closed` (normal shutdown
/// race) but **bubbles** `Saturated` — slow downstream silently
/// dropping events is the exact failure mode the metrics counters
/// can't catch.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SinkError {
    /// The sink's buffer / channel is full. Drop policy is the
    /// caller's; the broadcast blanket impl raises this rather than
    /// silently dropping the event.
    #[error("sink saturated: {kind}")]
    Saturated {
        /// The event kind that could not be delivered.
        kind: String,
    },

    /// The sink is permanently closed (receiver dropped, task gone).
    #[error("sink closed: {kind}")]
    Closed {
        /// The event kind that could not be delivered.
        kind: String,
    },

    /// Anything else — serde failure, downstream I/O, etc.
    #[error("sink failed: {0}")]
    Other(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Result alias for [`EventSink::emit`].
///
/// Sink errors are intentionally **not** folded into [`crate::Error`]:
/// they describe a back-pressure / liveness signal the service caller
/// has to act on (Decision D4), not a domain failure to map onto an
/// HTTP status. Keeping them on their own axis is the whole point.
pub type SinkResult<T> = std::result::Result<T, SinkError>;

/// Where a [`Service`](super::Service) sends events it observes.
///
/// `kind` is a stable, service-supplied string (e.g. `"slack.message"`).
/// `payload` is the deserialized provider event as JSON. Implementations
/// fan into whatever shape the consumer wants — a `broadcast::Sender`,
/// an `mpsc::Sender`, a webhook forwarder, a test double.
#[async_trait]
pub trait EventSink: Send + Sync + 'static {
    /// Publish an event. See [`SinkError`] for the failure shape.
    async fn emit(&self, kind: &str, payload: Value) -> SinkResult<()>;

    /// Compute a per-event dedup key, or `None` to fall back to a
    /// caller-supplied hash. Default impl returns `None` so existing
    /// implementations stay source-compatible.
    ///
    /// Per `DOCS/flow/scope/SCOPE.md` D-F3.12 (Phase 3 job): the
    /// `FlowAsService` wrapper uses this accessor to ask the sink
    /// "do you already know the semantic unit of work this event
    /// represents?" — typical implementations return the upstream
    /// provider's idempotency id (Slack `event_id`, Telegram
    /// `update_id`, an SQS message-deduplication-id, etc.). When
    /// the sink returns `None` the wrapper falls back to
    /// `blake3((service_name, kind, payload_bytes))`.
    ///
    /// At-least-once delivery is the only realistic contract over
    /// the workspace's four transports (REST SSE, MCP, gRPC
    /// streaming, JSON-RPC stdio — all of which can re-deliver
    /// after a reconnect). The dedup key makes that contract safe.
    #[allow(unused_variables)]
    fn dedup_key(&self, kind: &str, payload: &Value) -> Option<String> {
        None
    }
}
