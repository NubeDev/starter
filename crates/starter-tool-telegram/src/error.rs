//! [`TelegramError`] — the failure modes `sendMessage` produces.
//!
//! Lifted from `codeless-telegram::web_api::WebApiError`. The variant
//! shape is the same; only the conversion target changes — `From` lands
//! on [`starter_spi::Error`] so [`crate::TelegramSendMessageTool::invoke`]
//! can return `starter_spi::Result<Value>`.

use thiserror::Error;

/// Error surface for `sendMessage` calls.
#[derive(Debug, Error)]
pub enum TelegramError {
    /// Underlying transport failure (DNS, TLS, connection reset, …).
    #[error("sendMessage transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Telegram rate-limited the call. The Bot API surfaces this as a
    /// 429 response with a `retry_after` field in the JSON body; the
    /// header equivalent is also captured when present so a retry
    /// layer doesn't have to parse the raw response.
    #[error("sendMessage rate-limited (retry_after_secs={retry_after_secs:?})")]
    RateLimited {
        /// `Retry-After` seconds, if Telegram provided one.
        retry_after_secs: Option<u64>,
    },

    /// Telegram returned a non-2xx, non-429 status. Most commonly 5xx
    /// during platform incidents.
    #[error("sendMessage returned HTTP {status}")]
    HttpStatus {
        /// Raw HTTP status code from Telegram.
        status: u16,
    },

    /// Bot API returned 200-with-`ok: false`. The `description` label
    /// (`Forbidden: bot was blocked by the user`,
    /// `Bad Request: chat not found`, …) is propagated verbatim so
    /// operators / tests can grep for a specific failure mode.
    #[error("sendMessage returned ok=false (description={0:?})")]
    BotApi(Option<String>),

    /// `ok: true` arrived without a `result` object. Treated as a
    /// Telegram-side protocol break rather than silently dropped.
    #[error("sendMessage returned ok=true without a result")]
    MissingResult,
}

impl From<TelegramError> for starter_spi::Error {
    fn from(err: TelegramError) -> Self {
        match &err {
            // Telegram returns "Unauthorized" for a bad bot token —
            // both as the description on `ok=false` and as a 401.
            // Map both onto the SPI's `Unauthenticated` variant so an
            // MCP caller sees a recognisable failure.
            TelegramError::BotApi(Some(label))
                if label.to_ascii_lowercase().starts_with("unauthorized") =>
            {
                starter_spi::Error::Unauthenticated
            }
            TelegramError::HttpStatus { status: 401 } => starter_spi::Error::Unauthenticated,
            _ => starter_spi::Error::Internal {
                source: Box::new(err),
            },
        }
    }
}
