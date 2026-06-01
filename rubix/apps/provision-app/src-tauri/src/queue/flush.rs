//! Pieces the `queue_flush` command composes to replay queued tool
//! payloads. The replay LOOP lives in the command
//! (`commands/queue_flush.rs`) because it interleaves network awaits
//! with SQLite writes, and a rusqlite `Connection` is `!Sync` — so the
//! DB lock must be released around each await, not held across it. This
//! file holds the bits that don't await: the tool id, the report shape,
//! and the per-row DB mutations.

use rusqlite::Connection;
use serde::Serialize;

use crate::queue::error::QueueError;

/// Per-flush outcome handed back to the UI.
#[derive(Debug, Default, Serialize)]
pub struct FlushReport {
    /// Rows successfully provisioned and removed from the queue.
    pub flushed: usize,
    /// Rows that errored this run and remain queued.
    pub failed: usize,
    /// Rows left untouched because the loop stopped early (agent down).
    pub remaining: usize,
}

/// Stamp a row as errored with its reason; keeps it queued for retry.
pub fn mark_error(conn: &Connection, id: i64, reason: &str) -> Result<(), QueueError> {
    conn.execute(
        "UPDATE pending_tools SET status = 'error', last_error = ?2 WHERE id = ?1",
        rusqlite::params![id, reason],
    )?;
    Ok(())
}
