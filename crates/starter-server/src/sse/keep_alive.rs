//! Standard keep-alive policy: comment frame every 15 seconds.
//! Long enough not to spam, short enough to defeat most proxies'
//! idle-connection killers.

use std::time::Duration;

use axum::response::sse::KeepAlive;

/// Default keep-alive used by starter SSE endpoints.
pub fn keep_alive() -> KeepAlive {
    KeepAlive::new().interval(Duration::from_secs(15))
}
