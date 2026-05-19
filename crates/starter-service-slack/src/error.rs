//! [`SlackSocketModeError`] — the failure modes the socket-mode
//! connect+pump loop produces.
//!
//! Lifted in shape from `codeless-slack::socket_mode::SocketModeError`;
//! the conversion target is [`starter_spi::Error`] so the service's
//! `JoinHandle` can resolve to `SpiResult<()>` per
//! [`Service::start`](starter_spi::service::Service::start).

use thiserror::Error;

/// Error surface for the socket-mode connect + pump loop.
#[derive(Debug, Error)]
pub enum SlackSocketModeError {
    /// Underlying HTTP transport failure on `apps.connections.open`
    /// (DNS, TLS, connection reset, …).
    #[error("apps.connections.open transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// `apps.connections.open` returned a non-2xx status.
    #[error("apps.connections.open returned HTTP {status}")]
    HttpStatus {
        /// Raw HTTP status code from Slack.
        status: u16,
    },

    /// `apps.connections.open` returned `ok: false`. The label Slack
    /// supplied is propagated verbatim.
    #[error("apps.connections.open returned ok=false (error={0:?})")]
    SlackApi(Option<String>),

    /// `apps.connections.open` returned `ok: true` without a `url`,
    /// or with a malformed URL.
    #[error("malformed wss_url from apps.connections.open: {0}")]
    BadWssUrl(String),

    /// WebSocket transport failure (dial / read / write).
    #[error("websocket: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// The retry circuit tripped: too many consecutive failures.
    /// Carries the last error so the operator can see *why* the circuit
    /// blew.
    #[error("retry circuit tripped after {attempts} consecutive failures; last error: {last}")]
    CircuitTripped {
        /// How many consecutive failures preceded the trip.
        attempts: u32,
        /// `Display`-rendered last error. Stored as a string because
        /// `Self` is not `Clone`.
        last: String,
    },
}

impl From<SlackSocketModeError> for starter_spi::Error {
    fn from(err: SlackSocketModeError) -> Self {
        starter_spi::Error::Internal {
            source: Box::new(err),
        }
    }
}
