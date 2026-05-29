//! Bucket math: snap an instant to a bucket boundary; decompose a
//! `(from, to)` window into a list of buckets.
//!
//! Two important invariants:
//!
//! - Bucket boundaries always align to UTC (per the proposal's
//!   `align_to: utc` choice) so two requests in different timezones
//!   share the same bucket keys.
//! - The "tail" bucket (the one containing `to`) is reported with
//!   `is_tail = true` so the cache layer can apply `tail_ttl`
//!   instead of `body_ttl`.

use crate::spec::WindowedSpec;
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};

/// One bucket in a decomposed range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bucket {
    /// Inclusive UTC start of the bucket.
    pub start: DateTime<Utc>,
    /// Exclusive UTC end of the bucket.
    pub end: DateTime<Utc>,
    /// `true` for the bucket that contains the request's `to`
    /// timestamp — the volatile / open one.
    pub is_tail: bool,
}

impl Bucket {
    /// A short stable string key for this bucket (UTC RFC3339 start).
    /// Used to compose cache keys at the layer.
    pub fn key(&self) -> String {
        self.start.to_rfc3339()
    }
}

/// Snap an instant `t` down to the previous bucket boundary of size
/// `bucket`, aligned to UTC epoch.
pub fn snap_to_bucket(t: DateTime<Utc>, bucket: ChronoDuration) -> DateTime<Utc> {
    let bucket_secs = bucket.num_seconds().max(1);
    let t_secs = t.timestamp();
    let floor = t_secs - t_secs.rem_euclid(bucket_secs);
    Utc.timestamp_opt(floor, 0).single().unwrap_or(t)
}

/// Decompose the `[from, to]` window into bucket-aligned segments.
///
/// `from` is snapped down to a bucket boundary; the inclusive range
/// of buckets that touch the window is emitted. The bucket that
/// contains `to` is flagged `is_tail = true`.
///
/// When `from >= to`, returns an empty vec.
pub fn decompose(spec: &WindowedSpec, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Bucket> {
    let mut out = Vec::new();
    if from >= to {
        return out;
    }
    let bucket = spec.bucket;
    let from_snap = snap_to_bucket(from, bucket);
    let to_snap = snap_to_bucket(to, bucket);
    let mut cur = from_snap;
    while cur <= to_snap {
        let next = cur + bucket;
        out.push(Bucket {
            start: cur,
            end: next,
            is_tail: cur == to_snap,
        });
        cur = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn snap_floors_to_hour() {
        let t = ts("2026-05-29T03:47:21Z");
        let snapped = snap_to_bucket(t, ChronoDuration::hours(1));
        assert_eq!(snapped, ts("2026-05-29T03:00:00Z"));
    }

    #[test]
    fn decompose_7d_into_168_hourly_buckets_plus_tail() {
        let spec = WindowedSpec::hourly();
        let to = ts("2026-05-29T03:47:21Z");
        let from = to - ChronoDuration::hours(24 * 7);
        let buckets = decompose(&spec, from, to);
        // from-snap = (to - 7d) floored to hour; to-snap = 03:00.
        // Inclusive count = (to_snap - from_snap)/1h + 1 = 169.
        assert_eq!(buckets.len(), 169);
        assert!(buckets.last().unwrap().is_tail);
        assert_eq!(buckets.iter().filter(|b| b.is_tail).count(), 1);
    }

    #[test]
    fn decompose_empty_when_from_ge_to() {
        let spec = WindowedSpec::hourly();
        let t = ts("2026-05-29T00:00:00Z");
        assert!(decompose(&spec, t, t).is_empty());
    }

    #[test]
    fn bucket_key_is_stable_rfc3339() {
        let spec = WindowedSpec::hourly();
        let buckets = decompose(
            &spec,
            ts("2026-01-01T00:30:00Z"),
            ts("2026-01-01T01:30:00Z"),
        );
        assert_eq!(buckets[0].key(), "2026-01-01T00:00:00+00:00");
    }
}
