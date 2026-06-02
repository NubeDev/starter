//! `tool_dispatch` — generic proxy to `POST /api/v1/tools/{tool_id}`.
//! The UI calls every bc_* tool (bc_decode, bc_provision, bc_site_list,
//! …) through this one command; it injects the session cookie (from the
//! client jar) and the X-CSRF-Token header, and returns the tool's raw
//! structured JSON untouched.

use serde_json::Value;
use tauri::State;

use crate::agent::client::AgentClientState;
use crate::agent::error::AgentError;
use crate::agent::session::SessionState;
use crate::error::AppError;

#[tauri::command]
pub async fn tool_dispatch(
    client: State<'_, AgentClientState>,
    session: State<'_, SessionState>,
    tool_id: String,
    params: Value,
) -> Result<Value, AppError> {
    let (base, csrf) = {
        let s = session.0.lock().await;
        (s.base_url.clone(), s.csrf_token.clone())
    };
    let base = base.ok_or(AgentError::NotConfigured)?;
    let csrf = csrf.ok_or(AgentError::NotAuthenticated)?;

    let out = client.0.tool(&base, &csrf, &tool_id, &params).await?;
    Ok(out)
}
