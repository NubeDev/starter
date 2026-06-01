//! `queue_drop` — discard one pending row by id (manual "remove this
//! queued scan"). Returns true if a row was removed.

use tauri::State;

use crate::error::AppError;
use crate::queue::drop::drop_row;
use crate::queue::open::QueueDb;

#[tauri::command]
pub async fn queue_drop(db: State<'_, QueueDb>, id: String) -> Result<bool, AppError> {
    // The frontend types QueueItem.id as a string; the row id is an i64.
    let id: i64 = id
        .parse()
        .map_err(|_| AppError::input(format!("invalid queue id: {id}")))?;
    let conn = db.0.lock().await;
    let removed = drop_row(&conn, id)?;
    Ok(removed)
}
