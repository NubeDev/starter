//! `extend(cached_range, requested_range) -> missing_ranges`.
//!
//! The classic "I have 7 days, give me 90" delta-fetch helper. The
//! returned sub-ranges are gaps the caller must fetch fresh; the
//! cached range is reused for the overlap.

use chrono::{DateTime, Utc};

/// Inclusive-start, exclusive-end range over UTC time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// Inclusive start.
    pub start: DateTime<Utc>,
    /// Exclusive end.
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Construct.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// `true` when this range carries no time.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// Given `cached` (the range we already hold) and `requested` (the
/// range the caller wants), return the sub-ranges of `requested`
/// that are **not** in `cached` and must be fetched fresh.
///
/// All four overlap cases are handled — strict suffix, strict
/// prefix, complete cover, and disjoint.
pub fn extend(cached: TimeRange, requested: TimeRange) -> Vec<TimeRange> {
    if requested.is_empty() {
        return Vec::new();
    }
    if cached.is_empty() {
        return vec![requested];
    }
    // Disjoint: cached entirely before or after requested.
    if cached.end <= requested.start || cached.start >= requested.end {
        return vec![requested];
    }
    let mut out = Vec::new();
    if requested.start < cached.start {
        out.push(TimeRange::new(requested.start, cached.start));
    }
    if requested.end > cached.end {
        out.push(TimeRange::new(cached.end, requested.end));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn ts(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn extending_a_7d_to_90d_returns_the_missing_83d_prefix() {
        let now = ts("2026-05-29T00:00:00Z");
        let cached = TimeRange::new(now - Duration::days(7), now);
        let want = TimeRange::new(now - Duration::days(90), now);
        let missing = extend(cached, want);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].start, now - Duration::days(90));
        assert_eq!(missing[0].end, now - Duration::days(7));
    }

    #[test]
    fn full_cover_returns_no_missing_ranges() {
        let now = ts("2026-05-29T00:00:00Z");
        let cached = TimeRange::new(now - Duration::days(90), now);
        let want = TimeRange::new(now - Duration::days(7), now);
        assert!(extend(cached, want).is_empty());
    }

    #[test]
    fn disjoint_returns_full_request() {
        let cached = TimeRange::new(ts("2025-01-01T00:00:00Z"), ts("2025-02-01T00:00:00Z"));
        let want = TimeRange::new(ts("2026-01-01T00:00:00Z"), ts("2026-02-01T00:00:00Z"));
        assert_eq!(extend(cached, want), vec![want]);
    }

    #[test]
    fn split_returns_both_prefix_and_suffix() {
        let cached = TimeRange::new(ts("2026-05-10T00:00:00Z"), ts("2026-05-20T00:00:00Z"));
        let want = TimeRange::new(ts("2026-05-01T00:00:00Z"), ts("2026-05-31T00:00:00Z"));
        let missing = extend(cached, want);
        assert_eq!(missing.len(), 2);
        assert_eq!(
            missing[0],
            TimeRange::new(ts("2026-05-01T00:00:00Z"), ts("2026-05-10T00:00:00Z"))
        );
        assert_eq!(
            missing[1],
            TimeRange::new(ts("2026-05-20T00:00:00Z"), ts("2026-05-31T00:00:00Z"))
        );
    }
}
