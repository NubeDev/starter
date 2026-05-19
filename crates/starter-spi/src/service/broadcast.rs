//! Blanket [`EventSink`] impl on `tokio::sync::broadcast::Sender<E>`
//! for any consumer-defined `E: From<Event>`. Gated on the `broadcast`
//! Cargo feature (default-on — Decision D2).
//!
//! Mapping rationale (Decision D4):
//!
//! - `tokio::sync::broadcast::Sender::send` only returns `Err(_)` when
//!   there are no active receivers — that is, the channel is
//!   permanently closed for this batch of subscribers. We surface
//!   that as [`SinkError::Closed`].
//! - `tokio::sync::broadcast` overwrites the oldest message when slow
//!   receivers lag; the sender cannot directly observe a saturated
//!   buffer (lag is reported on the receive side via
//!   `RecvError::Lagged`). The blanket impl therefore does **not**
//!   raise [`SinkError::Saturated`] from a `broadcast::Sender` — the
//!   `Saturated` variant exists for sinks that can detect overflow
//!   synchronously (e.g. `mpsc::Sender::try_send` returning `Full`).
//!
//! The blanket impl is generic over the consumer's event type. The
//! conversion from `(kind, payload)` to that type goes through
//! [`Event`], so the consumer writes a single `From<Event> for MyEvent`
//! impl and gets sink-shape automatically.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::broadcast;

use super::event::Event;
use super::sink::{EventSink, SinkError, SinkResult};

#[async_trait]
impl<E> EventSink for broadcast::Sender<E>
where
    E: From<Event> + Clone + Send + Sync + 'static,
{
    async fn emit(&self, kind: &str, payload: Value) -> SinkResult<()> {
        let event = Event::new(kind, payload);
        let typed: E = event.into();
        match self.send(typed) {
            Ok(_) => Ok(()),
            Err(_) => Err(SinkError::Closed {
                kind: kind.to_string(),
            }),
        }
    }
}
