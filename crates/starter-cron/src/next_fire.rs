//! Compute the next scheduled fire time for a cron expression.
//!
//! Single public function — [`next_fire`] — so the scheduler loop and
//! its tests have exactly one knob to turn. The implementation is
//! deliberately allocation-light: we parse the expression once per
//! call, ask the [`cron`] crate for the first occurrence strictly
//! after `now`, and translate the two failure modes into a typed
//! [`CronError`].

use std::str::FromStr;

use chrono::{DateTime, Utc};
use cron::Schedule;

use crate::error::CronError;

/// Return the first scheduled fire instant strictly **after** `now`.
///
/// `expr` is parsed on every call — the scheduler is expected to
/// cache `Schedule` objects itself if it ever needs the throughput.
/// For the current call rate (one parse per scheduled row per tick)
/// the simpler signature wins.
///
/// # Errors
///
/// - [`CronError::Parse`] if `expr` is not a valid cron expression.
/// - [`CronError::Past`] if `expr` parses but has no occurrence after
///   `now` (e.g. a one-shot expression pinned to a past year).
pub fn next_fire(now: DateTime<Utc>, expr: &str) -> Result<DateTime<Utc>, CronError> {
    let schedule = Schedule::from_str(expr).map_err(|e| CronError::Parse {
        expr: expr.to_owned(),
        source: Box::new(e),
    })?;

    schedule.after(&now).next().ok_or_else(|| CronError::Past {
        expr: expr.to_owned(),
    })
}
