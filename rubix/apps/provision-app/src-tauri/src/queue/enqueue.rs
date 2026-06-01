//! Insert one pending tool replay. Called when a tool dispatch is
//! confirmed while the agent is unreachable (BARCODE.md §6.1 — offline
//! queue a scan and sync later). Generic: any `tool_id` + params.

use rusqlite::Connection;
use serde_json::Value;

use crate::queue::error::QueueError;
use crate::queue::row::PendingTool;

/// Persist `params` for `tool_id` as a new pending row, stamped with a
/// sortable creation timestamp. Returns the created `PendingTool` so the
/// command can hand the frontend a full `QueueItem`.
pub fn enqueue(
    conn: &Connection,
    tool_id: &str,
    params: &Value,
) -> Result<PendingTool, QueueError> {
    // Store verbatim so flush replays exactly what the UI built.
    let payload_json =
        serde_json::to_string(params).map_err(|e| QueueError::Payload(e.to_string()))?;
    let created_at = now_millis();

    conn.execute(
        "INSERT INTO pending_tools (tool_id, payload_json, created_at, status)
         VALUES (?1, ?2, ?3, 'pending')",
        rusqlite::params![tool_id, payload_json, created_at.to_string()],
    )?;

    Ok(PendingTool {
        id: conn.last_insert_rowid(),
        tool_id: tool_id.to_string(),
        params: params.clone(),
        enqueued_at: created_at as i64,
        status: "pending".to_string(),
        last_error: None,
    })
}

/// Millis since the Unix epoch, zero-padded by virtue of being a large
/// fixed-width integer so lexical TEXT ordering matches chronological.
fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
