//! `queue_list` — the pending offline-provision queue, oldest first.

use tauri::State;

use crate::error::AppError;
use crate::queue::list::list;
use crate::queue::open::QueueDb;
use crate::queue::row::PendingTool;

#[tauri::command]
pub async fn queue_list(db: State<'_, QueueDb>) -> Result<Vec<PendingTool>, AppError> {
    let conn = db.0.lock().await;
    let rows = list(&conn)?;
    Ok(rows)
}
