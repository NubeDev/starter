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
}
