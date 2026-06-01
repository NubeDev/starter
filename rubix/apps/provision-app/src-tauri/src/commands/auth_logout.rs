//! `auth_logout` — end the session on the agent (needs the CSRF header)
//! and clear the in-memory credentials. Idempotent: if there is no
//! session, it succeeds without a network call.

use tauri::State;

use crate::agent::client::AgentClientState;
use crate::agent::session::SessionState;
use crate::error::AppError;

#[tauri::command]
pub async fn auth_logout(
    client: State<'_, AgentClientState>,
    session: State<'_, SessionState>,
) -> Result<(), AppError> {
    // Snapshot what we need, then drop the guard before the await.
    let creds = {
        let s = session.0.lock().await;
        s.is_authenticated()
            .then(|| (s.base_url.clone(), s.csrf_token.clone()))
    };

    // Only hit the network if we actually hold a session.
    if let Some((Some(base), Some(csrf))) = creds {
        client.0.logout(&base, &csrf).await?;
    }

    session.0.lock().await.clear_auth();
    Ok(())
}
