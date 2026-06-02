//! `saved_base_url` — the agent base_url remembered from the last run, or
//! `null` if none was ever persisted. Read from the live session, which is
//! pre-filled at startup from the on-disk store (see `lib.rs` setup).
//!
//! The web transport can seed the Connect form synchronously from
//! localStorage, but on Tauri the URL lives in the Rust core — this command
//! is how the form hydrates that remembered host instead of reverting to the
//! compiled-in default on every launch.

use tauri::{AppHandle, Runtime, State};

use crate::agent::session::SessionState;
use crate::error::AppError;
use crate::store::base_url as base_url_store;

#[tauri::command]
pub async fn saved_base_url<R: Runtime>(
    app: AppHandle<R>,
    session: State<'_, SessionState>,
) -> Result<Option<String>, AppError> {
    // Prefer the live session, but fall back to reading the on-disk store
    // directly. On Android the webview (and the JS `invoke` from the Connect
    // form's mount effect) can fire BEFORE Tauri's `setup` hook has finished
    // seeding the session from the store — reading the store here closes that
    // race so the remembered host shows up on the very first paint.
    let from_session = {
        let s = session.0.lock().await;
        s.base_url.clone()
    };
    if let Some(base) = from_session {
        return Ok(Some(base));
    }
    Ok(base_url_store::load(&app))
}
