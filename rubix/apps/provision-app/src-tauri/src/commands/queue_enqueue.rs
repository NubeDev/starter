//! `queue_enqueue` — store a tool payload locally for later sync
//! (offline scan, BARCODE.md §6.1). Returns the created `QueueItem`.

use serde_json::Value;
use tauri::State;

use crate::error::AppError;
use crate::queue::enqueue::enqueue;
use crate::queue::open::QueueDb;
use crate::queue::row::PendingTool;

#[tauri::command]
pub async fn queue_enqueue(
    db: State<'_, QueueDb>,
    tool_id: String,
    params: Value,
) -> Result<PendingTool, AppError> {
    let conn = db.0.lock().await;
    let item = enqueue(&conn, &tool_id, &params)?;
    Ok(item)
}
