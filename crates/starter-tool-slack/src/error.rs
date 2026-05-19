//! [`SlackError`] — the failure modes `chat.postMessage` produces.
//!
//! Lifted from `codeless-slack::web_api::SlackPostError`. The variant
//! shape is the same; only the conversion target changes — `From` lands
//! on [`starter_spi::Error`] so [`crate::SlackPostTool::invoke`] can
//! return `starter_spi::Result<Value>`.

use thiserror::Error;

/// Error surface for `chat.postMessage` calls.
#[derive(Debug, Error)]
pub enum SlackError {
    /// Underlying transport failure (DNS, TLS, connection reset, …).
    #[error("chat.postMessage transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Slack rate-limited the call. The body is normally a `Retry-After`
    /// header; that header (in seconds) is surfaced here when present so
    /// the caller's retry/backoff layer can read it without parsing the
    /// raw response.
    #[error("chat.postMessage rate-limited (retry_after_secs={retry_after_secs:?})")]
    RateLimited {
        /// `Retry-After` header value in seconds, if Slack sent one.
        retry_after_secs: Option<u64>,
    },

    /// Slack returned a non-2xx, non-429 status. Most commonly 5xx
    /// during Slack-side incidents; surface it so the caller can
    /// distinguish "Slack is broken" from "we sent bad input".
    #[error("chat.postMessage returned HTTP {status}")]
    HttpStatus {
        /// Raw HTTP status code from Slack.
        status: u16,
    },

    /// Slack's API returns 200-with-`ok: false` plus an `error` label
    /// for permission / scope problems (`invalid_auth`, `not_in_channel`,
    /// `channel_not_found`, …). The label is propagated verbatim so the
    /// operator / tests can grep for a specific failure mode.
    #[error("chat.postMessage returned ok=false (error={0:?})")]
    SlackApi(Option<String>),

    /// `ok: true` arrived without the documented `ts` field. Treated
    /// as a Slack-side protocol break rather than silently dropped.
    #[error("chat.postMessage returned ok=true without a ts field")]
    MissingTs,
}

impl From<SlackError> for starter_spi::Error {
    fn from(err: SlackError) -> Self {
        match &err {
            // `invalid_auth` / `not_authed` are the auth-failure labels
            // Slack returns in the `ok=false` branch. Map them to the
            // SPI's `Unauthenticated` variant so an MCP caller sees a
            // recognisable failure rather than `internal error`.
            SlackError::SlackApi(Some(label))
                if label == "invalid_auth" || label == "not_authed" =>
            {
                starter_spi::Error::Unauthenticated
            }
            _ => starter_spi::Error::Internal {
                source: Box::new(err),
            },
        }
    }
}
