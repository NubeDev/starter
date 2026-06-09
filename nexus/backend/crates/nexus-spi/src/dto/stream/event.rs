//! The SSE event payload pushed to subscribers of a live stream.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// One `data:` frame on a live stream. `seq` is the monotonic per-stream event
/// id echoed in the SSE `id:` field so a reconnecting client can send
/// `Last-Event-ID` to resume; `rows` is the batch shaped by the stream's SQL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StreamEvent {
    /// Monotonic event sequence number within this stream.
    pub seq: u64,
    /// The rows in this batch, one JSON object per row.
    pub rows: Vec<Value>,
}
