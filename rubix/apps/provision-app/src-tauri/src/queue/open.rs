//! Open the offline-queue SQLite DB and migrate its one table.
//!
//! The DB lives under Tauri's app data dir so it is per-install and
//! survives restarts. rusqlite is bundled, so there is no system
//! libsqlite dependency on desktop or mobile.

use std::path::Path;

use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::queue::error::QueueError;

/// File name under the app data dir.
const DB_FILE: &str = "offline-queue.sqlite3";

/// Tauri managed state: the single connection behind an async mutex so
/// command handlers on different threads serialise their access.
pub struct QueueDb(pub Mutex<Connection>);

/// Open (creating if needed) the queue DB inside `app_data_dir` and run
/// the idempotent migration. Called once from `lib.rs` setup.
pub fn open(app_data_dir: &Path) -> Result<Connection, QueueError> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| QueueError::DataDir(e.to_string()))?;
    let conn = Connection::open(app_data_dir.join(DB_FILE))?;
    migrate(&conn)?;
    Ok(conn)
}

/// Create the `pending_tools` table if it is absent. The queue is
/// generic: each row replays its own `tool_id` with `payload_json` as
/// params. Schema: id PK, tool_id, payload_json, created_at, status,
/// last_error.
fn migrate(conn: &Connection) -> Result<(), QueueError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS pending_tools (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            tool_id     TEXT     NOT NULL,
            payload_json TEXT    NOT NULL,
            created_at  TEXT     NOT NULL,
            status      TEXT     NOT NULL DEFAULT 'pending',
            last_error  TEXT
        );",
    )?;
    Ok(())
}
