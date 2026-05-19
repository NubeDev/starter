//! [`TelegramBotError`] — the failure modes the long-poll loop
//! produces.
//!
//! Lifted in shape from `codeless-telegram::web_api::WebApiError`;
//! the conversion target is [`starter_spi::Error`] so the service's
//! `JoinHandle` can resolve to `SpiResult<()>` per
//! [`Service::start`](starter_spi::service::Service::start).

use thiserror::Error;

/// Error surface for the `getUpdates` long-poll loop.
#[derive(Debug, Error)]
pub enum TelegramBotError {
    /// Underlying transport failure on `getUpdates` (DNS, TLS,
    /// connection reset, …).
    #[error("getUpdates transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// `getUpdates` returned a non-2xx status. 401 / 404 are
    /// non-transient (bad token / wrong base URL) and trip the
    /// circuit immediately rather than waiting for `max_attempts`.
    #[error("getUpdates returned HTTP {status}")]
    HttpStatus {
        /// Raw HTTP status code from Telegram.
        status: u16,
    },

    /// Bot API returned 200-with-`ok: false`. Description (when
    /// present) is propagated verbatim.
    #[error("getUpdates returned ok=false (description={0:?})")]
    BotApi(Option<String>),

    /// The retry circuit tripped: too many consecutive failures, or a
    /// non-transient error like 401 / 404.
    #[error("retry circuit tripped after {attempts} consecutive failures; last error: {last}")]
    CircuitTripped {
        /// How many consecutive failures preceded the trip.
        attempts: u32,
        /// `Display`-rendered last error. Stored as a string because
        /// `Self` is not `Clone`.
        last: String,
    },
}

impl From<TelegramBotError> for starter_spi::Error {
    fn from(err: TelegramBotError) -> Self {
        starter_spi::Error::Internal {
            source: Box::new(err),
        }
    }
}

impl TelegramBotError {
    /// True for failures the retry layer should *not* sleep on — the
    /// circuit trips immediately. Bad token / wrong host: backing off
    /// forever just buries the cause.
    pub(crate) fn is_fatal(&self) -> bool {
        matches!(
            self,
            TelegramBotError::HttpStatus { status: 401 }
                | TelegramBotError::HttpStatus { status: 404 }
        )
    }
}
