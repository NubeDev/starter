//! `auth_me` — current identity, or `null` if not authenticated. Reads
//! the base_url from the live session; if no login has run, returns
//! `null` rather than erroring so the UI renders a logged-out state.

use serde_json::Value;
use tauri::State;

use crate::agent::client::AgentClientState;
use crate::agent::session::SessionState;
use crate::error::AppError;

#[tauri::command]
pub async fn auth_me(
    client: State<'_, AgentClientState>,
    session: State<'_, SessionState>,
) -> Result<Value, AppError> {
    let base = {
        let s = session.0.lock().await;
        s.base_url.clone()
    };
    let Some(base) = base else {
        return Ok(Value::Null);
    };
    let me = client.0.me(&base).await?;
    Ok(me.unwrap_or(Value::Null))
}
