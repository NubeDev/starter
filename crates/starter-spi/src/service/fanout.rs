//! [`FanOut`] — a `Vec<Arc<dyn EventSink>>` helper that emits to every
//! sink it owns and **logs-and-continues** on individual failures, but
//! bubbles a `Saturated` back to the caller so the service can apply
//! back-pressure on its upstream (Decision D4).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tracing::warn;

use super::sink::{EventSink, SinkError, SinkResult};

/// Fan-out helper that publishes each event to every contained sink.
///
/// Failure policy (Decision D4):
///
/// - `Ok(())` from every sink ⇒ `Ok(())`.
/// - One or more sinks return `SinkError::Closed { .. }` ⇒ logged at
///   `warn`, skipped. A closed sink is a normal shutdown race, not a
///   back-pressure signal.
/// - One or more sinks return `SinkError::Other(_)` ⇒ logged at
///   `warn`, skipped. One broken downstream must not gag the rest.
/// - **Any** sink returns `SinkError::Saturated { .. }` ⇒ after the
///   fan-out completes, a `Saturated` is bubbled to the caller. The
///   service is expected to slow its upstream (stop ack'ing, pause
///   the poll loop).
#[derive(Default, Clone)]
pub struct FanOut {
    sinks: Vec<Arc<dyn EventSink>>,
}

impl FanOut {
    /// Empty fan-out.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a sink. Order is delivery order, but a slow sink does
    /// not block the next — sinks are awaited sequentially because the
    /// trait's `emit` is `async`; consumers who need concurrent
    /// dispatch should wrap each sink in its own spawned task.
    pub fn with(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    /// Mutable in-place append, for builder loops.
    pub fn push(&mut self, sink: Arc<dyn EventSink>) {
        self.sinks.push(sink);
    }

    /// Number of contained sinks.
    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    /// Whether the fan-out is empty.
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

#[async_trait]
impl EventSink for FanOut {
    async fn emit(&self, kind: &str, payload: Value) -> SinkResult<()> {
        let mut saturated = false;
        for (idx, sink) in self.sinks.iter().enumerate() {
            match sink.emit(kind, payload.clone()).await {
                Ok(()) => {}
                Err(SinkError::Saturated { kind: k }) => {
                    warn!(
                        target: "starter_spi::service::fanout",
                        sink_index = idx,
                        event.kind = %k,
                        "fan-out sink saturated; will bubble after fan-out completes"
                    );
                    saturated = true;
                }
                Err(SinkError::Closed { kind: k }) => {
                    warn!(
                        target: "starter_spi::service::fanout",
                        sink_index = idx,
                        event.kind = %k,
                        "fan-out sink closed; skipping"
                    );
                }
                Err(SinkError::Other(err)) => {
                    warn!(
                        target: "starter_spi::service::fanout",
                        sink_index = idx,
                        event.kind = %kind,
                        error = %err,
                        "fan-out sink failed; skipping"
                    );
                }
            }
        }

        if saturated {
            Err(SinkError::Saturated {
                kind: kind.to_string(),
            })
        } else {
            Ok(())
        }
    }
}
