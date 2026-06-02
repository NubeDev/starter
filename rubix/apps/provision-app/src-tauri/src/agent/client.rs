//! Thin reqwest wrapper around the rubix-agent REST API.
//!
//! The client owns a persistent cookie jar so the session cookie set by
//! `POST /auth/login` rides along on every later request automatically.
//! It is intentionally dumb transport: build a URL, send, map status to
//! `AgentError`, return JSON. All credential/state lives in `Session`
//! (managed state); this struct is shared and stateless beyond the jar.

use std::sync::Arc;

use reqwest::Client;
use serde_json::Value;

use crate::agent::error::AgentError;

/// Header the agent reads for the CSRF double-submit token.
const CSRF_HEADER: &str = "X-CSRF-Token";

/// Shared HTTP client + cookie jar. Cheap to clone (Arc inside).
#[derive(Clone)]
pub struct AgentClient {
    http: Client,
}

/// Tauri managed state holder for the single shared client.
pub struct AgentClientState(pub Arc<AgentClient>);

impl AgentClient {
    /// Build the client with an in-memory cookie store enabled. One per
    /// app process; the jar persists the session for the run.
    pub fn new() -> Result<Self, AgentError> {
        let http = Client::builder()
            .cookie_store(true)
            .user_agent("rubix-provision/0.1")
            .build()
            .map_err(AgentError::from)?;
        Ok(Self { http })
    }

    /// `POST {base}/api/v1/auth/login` — sets the session cookie in the
    /// jar and returns the CSRF token. Maps 401 to InvalidCredentials.
    pub async fn login(
        &self,
        base_url: &str,
        email: &str,
        password: &str,
    ) -> Result<String, AgentError> {
        let url = endpoint(base_url, "auth/login");
        let resp = self
            .http
            .post(url)
            .json(&serde_json::json!({ "email": email, "password": password }))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AgentError::InvalidCredentials);
        }
        let body = ok_json(resp).await?;
        body.get("csrf_token")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| AgentError::Decode("login response missing csrf_token".into()))
    }

    /// `GET {base}/api/v1/auth/me` — current identity. Returns Ok(None)
    /// on 401 (not authenticated) so the UI can render a logged-out state.
    pub async fn me(&self, base_url: &str) -> Result<Option<Value>, AgentError> {
        let url = endpoint(base_url, "auth/me");
        let resp = self.http.get(url).send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Ok(None);
        }
        ok_json(resp).await.map(Some)
    }

    /// `POST {base}/api/v1/auth/logout` — needs the CSRF header. 204 is
    /// the success case (no body); idempotent.
    pub async fn logout(&self, base_url: &str, csrf: &str) -> Result<(), AgentError> {
        let url = endpoint(base_url, "auth/logout");
        let resp = self
            .http
            .post(url)
            .header(CSRF_HEADER, csrf)
            .send()
            .await?;
        ok_empty(resp).await
    }

    /// `POST {base}/api/v1/tools/{tool_id}` with `params` as the JSON
    /// body — the generic bc_* proxy. Cookie rides from the jar; CSRF
    /// header echoes the token for the mutating call.
    pub async fn tool(
        &self,
        base_url: &str,
        csrf: &str,
        tool_id: &str,
        params: &Value,
    ) -> Result<Value, AgentError> {
        let url = endpoint(base_url, &format!("tools/{tool_id}"));
        let resp = self
            .http
            .post(url)
            .header(CSRF_HEADER, csrf)
            .json(params)
            .send()
            .await?;
        ok_json(resp).await
    }
}

/// Join `base_url` + the versioned API prefix + a relative path.
fn endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/api/v1/{path}")
}

/// Map a response to its JSON body, turning non-2xx into a Status error
/// carrying the body text so the UI sees the agent's real message.
async fn ok_json(resp: reqwest::Response) -> Result<Value, AgentError> {
    let status = resp.status();
    if status.is_success() {
        return resp.json().await.map_err(AgentError::from);
    }
    let body = resp.text().await.unwrap_or_default();
    Err(AgentError::Status {
        status: status.as_u16(),
        body,
    })
}

/// Like `ok_json` but for endpoints that return no body (logout → 204).
async fn ok_empty(resp: reqwest::Response) -> Result<(), AgentError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    Err(AgentError::Status {
        status: status.as_u16(),
        body,
    })
}
