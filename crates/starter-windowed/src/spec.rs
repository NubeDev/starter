//! `WindowedSpec` — declarative shape pinned by the proposal
//! `Layer 4b — time-windowed reads (the dashboard workload)`.

use chrono::Duration as ChronoDuration;
use serde::{Deserialize, Serialize};

/// What boundary the bucket snaps to. UTC is the only sane choice
/// when distinct user-locale timezones must share the same cached
/// bucket — different rendering tz, same physical bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignTo {
    /// Snap to UTC boundaries.
    Utc,
}

impl Default for AlignTo {
    fn default() -> Self {
        AlignTo::Utc
    }
}

/// Spec for a windowed read.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowedSpec {
    /// Request param carrying the upper-bound timestamp ("now").
    pub time_param: String,
    /// Request param carrying the window start.
    pub range_param: String,
    /// Bucket size. Each closed bucket is immutable; the open
    /// (tail) bucket containing `now` is volatile.
    pub bucket: ChronoDuration,
    /// TTL for the open tail bucket (volatile).
    pub tail_ttl: std::time::Duration,
    /// TTL for closed historical buckets (effectively immutable).
    pub body_ttl: std::time::Duration,
    /// Bucket boundary alignment. UTC for now.
    pub align_to: AlignTo,
}

impl WindowedSpec {
    /// Convenience constructor for the canonical hourly spec the
    /// dashboard workload uses.
    pub fn hourly() -> Self {
        Self {
            time_param: "to".into(),
            range_param: "from".into(),
            bucket: ChronoDuration::hours(1),
            tail_ttl: std::time::Duration::from_secs(30),
            body_ttl: std::time::Duration::from_secs(86_400),
            align_to: AlignTo::Utc,
        }
    }

    /// Override the tail TTL.
    pub fn tail_ttl(mut self, d: std::time::Duration) -> Self {
        self.tail_ttl = d;
        self
    }

    /// Override the body TTL.
    pub fn body_ttl(mut self, d: std::time::Duration) -> Self {
        self.body_ttl = d;
        self
    }

    /// Override the bucket size.
    pub fn bucket(mut self, d: ChronoDuration) -> Self {
        self.bucket = d;
        self
    }
}
