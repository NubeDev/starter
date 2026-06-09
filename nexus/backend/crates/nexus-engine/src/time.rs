//! Parse the human interval strings flow configs use (`"15m"`, `"30s"`, `"1h"`).
//!
//! Flow configs are authored by users, not code, so the cadence is a friendly
//! string rather than a raw second count. The grammar is intentionally tiny —
//! an integer followed by one of `s`/`m`/`h` — because a flow's poll interval
//! never needs sub-second or calendar precision.

use std::time::Duration;

/// Parse an interval like `"15m"` into a [`Duration`]. Returns a message naming
/// the offending input on a malformed value so a saved flow's bad cadence is a
/// clear config error, not a silent default.
pub fn parse_interval(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    let (num, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| format!("interval `{s}` has no unit (expected s/m/h)"))?,
    );
    let n: u64 = num
        .parse()
        .map_err(|_| format!("interval `{s}` has a non-numeric amount"))?;
    let secs = match unit {
        "s" => n,
        "m" => n * 60,
        "h" => n * 3600,
        other => return Err(format!("interval `{s}` has unknown unit `{other}` (expected s/m/h)")),
    };
    if secs == 0 {
        return Err(format!("interval `{s}` must be greater than zero"));
    }
    Ok(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_interval("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_interval("15m").unwrap(), Duration::from_secs(900));
        assert_eq!(parse_interval("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse_interval("15").is_err(), "no unit");
        assert!(parse_interval("xm").is_err(), "non-numeric");
        assert!(parse_interval("5d").is_err(), "unknown unit");
        assert!(parse_interval("0s").is_err(), "zero");
    }
}
