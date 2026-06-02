//! Errors raised by the agent transport (login, /me, logout, tool
//! dispatch). Kept separate from queue + app-facing errors so the
//! blast radius of a transport change stays inside this module.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    /// No `auth_login` has run yet, so there is no base_url to talk to.
    #[error("not configured: call auth_login with a base_url first")]
    NotConfigured,

    /// The agent rejected the credentials (HTTP 401 on login).
    #[error("invalid credentials")]
    InvalidCredentials,

    /// A mutating call was attempted without a session (no cookie/csrf).
    #[error("not authenticated: log in first")]
    NotAuthenticated,

    /// The agent answered with a non-success status; carries the code
    /// and any body text so the UI can surface the real reason.
    #[error("agent returned {status}: {body}")]
    Status { status: u16, body: String },

    /// Underlying reqwest transport failure (DNS, TLS, connect, timeout).
    #[error("transport error: {0}")]
    Transport(String),

    /// Response body was not the JSON we expected.
    #[error("decode error: {0}")]
    Decode(String),
}

impl From<reqwest::Error> for AgentError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            AgentError::Decode(e.to_string())
        } else {
            AgentError::Transport(e.to_string())
        }
    }
}
