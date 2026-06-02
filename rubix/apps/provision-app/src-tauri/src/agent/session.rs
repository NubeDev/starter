//! The live agent session: the base_url we proxy to plus the CSRF
//! token from a successful login. The session cookie itself lives in
//! reqwest's cookie jar (see `client.rs`); this struct only holds the
//! double-submit CSRF token, which is not a cookie and must be echoed
//! back as `X-CSRF-Token` on mutating requests (openapi: logout, tools).
//!
//! Wrapped in a `Mutex` inside Tauri managed state so commands on
//! different threads can read/replace it safely.

use tokio::sync::Mutex;

/// What `auth_login` establishes and later calls read.
#[derive(Debug, Default, Clone)]
pub struct Session {
    /// e.g. `http://127.0.0.1:8088` — no trailing slash, no `/api/...`.
    pub base_url: Option<String>,
    /// CSRF double-submit token from `LoginResponse.csrf_token`.
    pub csrf_token: Option<String>,
}

impl Session {
    /// True once a login has set both a base_url and a csrf token.
    pub fn is_authenticated(&self) -> bool {
        self.base_url.is_some() && self.csrf_token.is_some()
    }

    /// Clear credentials on logout, but keep `base_url` so the next
    /// login defaults to the same agent.
    pub fn clear_auth(&mut self) {
        self.csrf_token = None;
    }
}

/// Tauri managed state holder.
#[derive(Debug, Default)]
pub struct SessionState(pub Mutex<Session>);
