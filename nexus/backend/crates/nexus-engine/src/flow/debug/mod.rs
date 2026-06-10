//! Flow debug & values: the per-flow broadcast channel, the node tap decorators,
//! and a helper to publish log lines from the metered wrappers and the manager.
//!
//! When debug is enabled on a running flow, each node's [`tap`] publishes counter
//! ticks and sampled rows to the flow's [`channel::FlowDebugChannel`]; retries,
//! policy drops/DLQ, and the run-ending error publish [`FlowDebugEvent::Log`]
//! lines. The API exposes all of this over SSE.

pub mod channel;
pub mod tap;

use std::time::{SystemTime, UNIX_EPOCH};

use nexus_spi::dto::flow::{FlowDebugEvent, LogLevel};

pub use channel::{close, lookup, open, FlowDebugChannel, DEFAULT_SAMPLE_ROWS};
pub use tap::{with_debug, DebugProcessor, DebugSink, DebugSource};

/// Publish a log line to `flow_id`'s debug channel if it is running. A no-op when
/// the flow has no channel (debug stream never opened) — callers can fire this
/// unconditionally from the hot path's error branch. The line is published even
/// when sample capture is disabled, so an operator watching the log tab sees
/// retries/drops without enabling row sampling.
pub fn log(flow_id: &str, level: LogLevel, node_index: Option<u32>, message: impl Into<String>) {
    if let Some(channel) = lookup(flow_id) {
        let at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        channel.publish(FlowDebugEvent::Log {
            seq: channel.next_seq(),
            level,
            node_index,
            message: message.into(),
            at_ms,
        });
    }
}
