//! `saved_base_url` — the agent base_url remembered from the last run, or
//! `null` if none was ever persisted. Read from the live session, which is
//! pre-filled at startup from the on-disk store (see `lib.rs` setup).
//!
//! The web transport can seed the Connect form synchronously from
//! localStorage, but on Tauri the URL lives in the Rust core — this command
//! is how the form hydrates that remembered host instead of reverting to the
//! compiled-in default on every launch.

use tauri::State;

use crate::agent::session::SessionState;
use crate::error::AppError;

#[tauri::command]
pub async fn saved_base_url(session: State<'_, SessionState>) -> Result<Option<String>, AppError> {
    let s = session.0.lock().await;
    Ok(s.base_url.clone())
}
