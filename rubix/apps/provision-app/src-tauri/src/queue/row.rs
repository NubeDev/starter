//! The shape of one queued tool replay as it crosses to the frontend.
//! Mirrors the `pending_tools` table (see `open.rs`) and serialises to
//! the frontend's `QueueItem`:
//! `{ id: string, tool_id: string, params: <json>, enqueued_at: number }`.

use serde::{Serialize, Serializer};
use serde_json::Value;

/// A single queued tool replay awaiting flush.
#[derive(Debug, Clone, Serialize)]
pub struct PendingTool {
    /// Row id. The frontend types this as a string, so emit it as one.
    #[serde(serialize_with = "id_as_string")]
    pub id: i64,
    /// Tool id this row replays against (e.g. a `bc_*` provision tool).
    pub tool_id: String,
    /// Structured params (parsed from the stored `payload_json`) so the
    /// frontend receives an object, not a JSON string.
    pub params: Value,
    /// Creation timestamp as millis since the Unix epoch (a JS number).
    pub enqueued_at: i64,
    /// `pending` | `error` — `error` rows stay queued for a retry. Extra
    /// key beyond the frontend contract; harmless.
    pub status: String,
    /// Last failure reason, set when a flush attempt fails. Extra key.
    pub last_error: Option<String>,
}

/// Serialise the i64 row id as a JSON string (frontend QueueItem.id).
fn id_as_string<S: Serializer>(id: &i64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&id.to_string())
}
