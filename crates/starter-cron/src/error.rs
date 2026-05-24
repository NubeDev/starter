//! Error surface for [`crate::next_fire`].

use thiserror::Error;

/// Things that can go wrong when parsing a cron expression or asking
/// it for the next fire time.
///
/// Two cases on purpose — they map to genuinely different operator
/// responses:
///
/// - [`CronError::Parse`] — the schedule string itself is broken.
///   The scheduled flow can never run; the row needs to be edited or
///   deleted.
/// - [`CronError::Past`] — the expression parses but yields no future
///   fire time from `now` (e.g. `* * * * * * 2020`). The schedule is
///   effectively retired; treat it as a "done" row.
#[derive(Debug, Error)]
pub enum CronError {
    /// The supplied expression failed to parse.
    #[error("invalid cron expression `{expr}`: {source}")]
    Parse {
        /// The offending expression, captured verbatim so callers can
        /// surface it in logs / API errors without re-threading it.
        expr: String,
        /// Underlying error from the `cron` crate, kept boxed-as-string
        /// so we don't leak the upstream type into our public API.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Expression parses but has no upcoming fire time after `now`.
    #[error("cron expression `{expr}` has no fire time after the supplied instant")]
    Past {
        /// The expression that ran out of future fire times.
        expr: String,
    },
}
