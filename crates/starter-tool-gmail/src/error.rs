//! [`GmailError`] — the failure modes `users.messages.send` produces.
//!
//! Lifted from `codeless-tools::email::mailer::MailerError` with the
//! variant shape adapted to the starter `Tool` boundary. `From` lands
//! on [`starter_spi::Error`] so [`crate::GmailSendTool::invoke`] can
//! return `starter_spi::Result<Value>`.

use thiserror::Error;

use crate::message::MessageError;

/// Error surface for `users.messages.send` calls.
#[derive(Debug, Error)]
pub enum GmailError {
    /// Underlying transport failure (DNS, TLS, connection reset, …).
    #[error("gmail.send transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Gmail returned 401 (`Authorization` missing/expired) or 403
    /// (`gmail.send` scope not granted). Both collapse into a single
    /// `Unauthenticated` SPI variant — a 401 wrapper layer that
    /// refreshes tokens cares about the *category*, not the
    /// difference between the two.
    #[error("gmail.send auth rejected (status={status}, body={body:?})")]
    Auth {
        /// Raw HTTP status code (401 or 403).
        status: u16,
        /// Best-effort response body for debugging; usually a
        /// `{"error": {…}}` envelope. Trimmed to a reasonable size
        /// by Gmail itself — we propagate verbatim.
        body: String,
    },

    /// Gmail returned a non-2xx, non-401/403 status. Most commonly
    /// 5xx during platform incidents, occasionally 400 for malformed
    /// `raw` payloads we somehow let through.
    #[error("gmail.send returned HTTP {status} (body={body:?})")]
    HttpStatus {
        /// Raw HTTP status code from Gmail.
        status: u16,
        /// Best-effort response body for debugging.
        body: String,
    },

    /// Gmail returned a 2xx response that didn't decode into a
    /// `{"id": "…"}` envelope. Treated as a Gmail-side protocol
    /// break rather than silently dropped.
    #[error("gmail.send returned a 2xx without an `id` field")]
    MissingId,

    /// The caller-supplied [`crate::GmailMessage`] could not be
    /// rendered to RFC 5322. Surfaces as
    /// [`starter_spi::Error::Invalid`] so the caller can fix the
    /// request.
    #[error("gmail.send message build: {0}")]
    Build(#[from] MessageError),
}

impl From<GmailError> for starter_spi::Error {
    fn from(err: GmailError) -> Self {
        match &err {
            GmailError::Auth { .. } => starter_spi::Error::Unauthenticated,
            GmailError::Build(_) => starter_spi::Error::Invalid {
                message: err.to_string(),
            },
            _ => starter_spi::Error::Internal {
                source: Box::new(err),
            },
        }
    }
}
