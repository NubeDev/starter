//! List pending tool replays in creation order (oldest first) — the same
//! order `flush` replays them, so the UI shows the sync queue truthfully.

use rusqlite::Connection;
use serde_json::Value;

use crate::queue::error::QueueError;
use crate::queue::row::PendingTool;

/// Read every queued row, oldest first.
pub fn list(conn: &Connection) -> Result<Vec<PendingTool>, QueueError> {
    let mut stmt = conn.prepare(
        "SELECT id, tool_id, payload_json, created_at, status, last_error
         FROM pending_tools
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            // Tuple of raw columns; JSON/number parsing happens after the
            // closure so a parse failure becomes a QueueError, not a
            // rusqlite error.
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(|(id, tool_id, payload_json, created_at, status, last_error)| {
            let params: Value = serde_json::from_str(&payload_json)
                .map_err(|e| QueueError::Payload(e.to_string()))?;
            let enqueued_at = created_at.parse::<i64>().unwrap_or(0);
            Ok(PendingTool {
                id,
                tool_id,
                params,
                enqueued_at,
                status,
                last_error,
            })
        })
        .collect()
}
