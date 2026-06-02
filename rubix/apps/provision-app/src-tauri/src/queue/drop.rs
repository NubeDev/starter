//! Remove one pending row by id — used after a successful flush and for
//! a manual "discard this queued scan" action in the UI.

use rusqlite::Connection;

use crate::queue::error::QueueError;

/// Delete the row with `id`. Returns true if a row was actually removed
/// (false means it was already gone — a no-op, not an error).
pub fn drop_row(conn: &Connection, id: i64) -> Result<bool, QueueError> {
    let affected = conn.execute(
        "DELETE FROM pending_tools WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(affected > 0)
}
