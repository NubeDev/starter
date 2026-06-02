//! `queue_flush` — replay every pending tool against the agent in
//! created order, each row POSTed to its own `tool_id`. Drops rows that
//! succeed, marks rows that fail. Relies on tool idempotency so a retry
//! is safe.
//!
//! The loop lives here (not in `queue::flush`) so the SQLite lock is
//! released around each network await — rusqlite's `Connection` is
//! `!Sync` and cannot be borrowed across `.await`.

use tauri::State;

use crate::agent::client::AgentClientState;
use crate::agent::error::AgentError;
use crate::agent::session::SessionState;
use crate::error::AppError;
use crate::queue::drop::drop_row;
use crate::queue::flush::{mark_error, FlushReport};
use crate::queue::list::list;
use crate::queue::open::QueueDb;

#[tauri::command]
pub async fn queue_flush(
    client: State<'_, AgentClientState>,
    session: State<'_, SessionState>,
    db: State<'_, QueueDb>,
) -> Result<FlushReport, AppError> {
    // Must be logged in to replay — surfaces as an `agent` error so the
    // UI can prompt a login before retrying.
    let (base, csrf) = {
        let s = session.0.lock().await;
        (s.base_url.clone(), s.csrf_token.clone())
    };
    let base = base.ok_or(AgentError::NotConfigured)?;
    let csrf = csrf.ok_or(AgentError::NotAuthenticated)?;
    let client = client.0.clone();

    // Snapshot the queue once (lock released immediately).
    let rows = {
        let conn = db.0.lock().await;
        list(&conn)?
    };
    let total = rows.len();
    let mut report = FlushReport::default();

    for (idx, row) in rows.into_iter().enumerate() {
        // Network await holds NO db lock. Each row replays its own tool.
        let result = client
            .tool(&base, &csrf, &row.tool_id, &row.params)
            .await;

        match result {
            Ok(_) => {
                let conn = db.0.lock().await;
                drop_row(&conn, row.id)?;
                report.flushed += 1;
            }
            // Agent unreachable / not configured: stop early, leave the
            // rest pending — no point hammering a down agent.
            Err(e @ (AgentError::Transport(_) | AgentError::NotConfigured)) => {
                let conn = db.0.lock().await;
                mark_error(&conn, row.id, &e.to_string())?;
                report.failed += 1;
                report.remaining = total - idx - 1;
                return Ok(report);
            }
            // Per-row failure (agent 4xx/5xx, decode): record and move
            // on so one bad scan doesn't block the rest of the queue.
            Err(e) => {
                let conn = db.0.lock().await;
                mark_error(&conn, row.id, &e.to_string())?;
                report.failed += 1;
            }
        }
    }
    Ok(report)
}
