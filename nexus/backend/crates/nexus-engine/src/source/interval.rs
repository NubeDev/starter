//! Parse the cadence strings native sources accept (`"5ms"`, `"30s"`, `"15m"`,
//! `"1h"`).
//!
//! The sources extend [`crate::time::parse_interval`] with a millisecond unit,
//! which the `generate` source needs for its fast test cadences (`"5ms"`). Kept
//! here, in the source lane, so the shared `time::parse_interval` stays a plain
//! second/minute/hour parser.

use std::time::Duration;

/// Parse a cadence like `"5ms"`, `"30s"`, `"15m"`, or `"1h"` into a [`Duration`].
/// Returns a message naming the offending input on a malformed or zero value so a
/// saved flow's bad cadence is a clear config error, not a silent default.
pub fn parse_cadence(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let split = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("interval `{s}` has no unit (expected ms/s/m/h)"))?;
    let (num, unit) = s.split_at(split);
    let n: u64 = num
        .parse()
        .map_err(|_| format!("interval `{s}` has a non-numeric amount"))?;
    let duration = match unit {
        "ms" => Duration::from_millis(n),
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        "h" => Duration::from_secs(n * 3600),
        other => {
            return Err(format!(
                "interval `{s}` has unknown unit `{other}` (expected ms/s/m/h)"
            ))
        }
    };
    if duration.is_zero() {
        return Err(format!("interval `{s}` must be greater than zero"));
    }
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_unit() {
        assert_eq!(parse_cadence("5ms").unwrap(), Duration::from_millis(5));
        assert_eq!(parse_cadence("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_cadence("15m").unwrap(), Duration::from_secs(900));
        assert_eq!(parse_cadence("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_cadence("15").is_err(), "no unit");
        assert!(parse_cadence("xs").is_err(), "non-numeric");
        assert!(parse_cadence("5d").is_err(), "unknown unit");
        assert!(parse_cadence("0s").is_err(), "zero");
    }
}
