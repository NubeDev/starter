//! `auth_login` — log into a rubix-agent: store the session cookie (in
//! the client jar) + CSRF token (in managed Session state), persist the
//! base_url, and return the caller's identity from `/auth/me`.

use serde_json::Value;
use tauri::{AppHandle, Runtime, State};

use crate::agent::client::AgentClientState;
use crate::agent::session::SessionState;
use crate::error::AppError;
use crate::store::base_url;

#[tauri::command]
pub async fn auth_login<R: Runtime>(
    app: AppHandle<R>,
    client: State<'_, AgentClientState>,
    session: State<'_, SessionState>,
    base_url_arg: String,
    email: String,
    password: String,
) -> Result<Value, AppError> {
    let base = base_url_arg.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err(AppError::input("base_url must not be empty"));
    }

    let client = client.0.clone();
    let csrf = client.login(&base, &email, &password).await?;

    // Establish the in-memory session before the /me round-trip.
    {
        let mut s = session.0.lock().await;
        s.base_url = Some(base.clone());
        s.csrf_token = Some(csrf);
    }
    base_url::save(&app, &base);

    // Return identity so the UI lands logged-in without a second call.
    let me = client.me(&base).await?;
    Ok(me.unwrap_or(Value::Null))
}
