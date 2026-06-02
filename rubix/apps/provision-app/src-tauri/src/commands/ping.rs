//! `ping` — a pre-login connectivity probe. Hits `GET {base}/healthz`
//! (host root, no auth) so the Connect screen can tell "agent
//! unreachable / wrong host" apart from "bad credentials". Unlike
//! `auth_login`, it does NOT touch the session — you can ping any host
//! without committing to it.

use serde::Serialize;
use tauri::State;

use crate::agent::client::AgentClientState;
use crate::agent::error::AgentError;
use crate::error::AppError;

/// Default ping timeout: short enough that a typo'd IP fails fast.
const PING_TIMEOUT_MS: u64 = 4000;

/// Result of a ping. `ok` is the headline the UI branches on; the other
/// fields give a human-readable detail line either way.
#[derive(Debug, Serialize)]
pub struct PingResult {
    /// True when `/healthz` answered 2xx.
    pub ok: bool,
    /// Round-trip time in ms on success; `null` on failure.
    pub latency_ms: Option<u128>,
    /// "reachable in 12 ms" or the concrete failure reason.
    pub message: String,
}

#[tauri::command]
pub async fn ping(
    client: State<'_, AgentClientState>,
    base_url: String,
) -> Result<PingResult, AppError> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err(AppError::input("base_url must not be empty"));
    }

    match client.0.ping(&base, PING_TIMEOUT_MS).await {
        Ok(latency) => Ok(PingResult {
            ok: true,
            latency_ms: Some(latency),
            message: format!("reachable in {latency} ms"),
        }),
        // A reachable-but-unhappy agent (non-2xx) still proves the host
        // is up, so report it as a soft failure with the status detail
        // rather than a transport error.
        Err(AgentError::Status { status, .. }) => Ok(PingResult {
            ok: false,
            latency_ms: None,
            message: format!("agent answered HTTP {status} (host reachable)"),
        }),
        Err(AgentError::Transport(detail)) => Ok(PingResult {
            ok: false,
            latency_ms: None,
            message: format!("cannot reach {base}: {detail}"),
        }),
        Err(other) => Ok(PingResult {
            ok: false,
            latency_ms: None,
            message: other.to_string(),
        }),
    }
}
