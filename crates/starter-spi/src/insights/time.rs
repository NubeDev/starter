//! [`TimeZoneId`] and [`Window`] — the timing types every
//! `Verdict` / `Dataset` carries (Insights SCOPE R-ins-6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// IANA time-zone identifier, e.g. `"Europe/London"` or `"UTC"`.
///
/// Phase 1 ships a string newtype; future phases may validate
/// against the IANA database at construction time. Stored verbatim
/// in the verdict log so DST-sensitive comparisons (this Tuesday vs
/// last Tuesday) survive replay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimeZoneId(pub String);

impl TimeZoneId {
    /// Construct a [`TimeZoneId`].
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// UTC, the default for tests and pipelines that don't care.
    pub fn utc() -> Self {
        Self("UTC".to_owned())
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A time window — `[start, end)`, UTC instants — paired on the
/// `Verdict`/`Dataset` with the [`TimeZoneId`] it was computed
/// against. Phase 1 uses windows only on `Verdict.window`; the
/// windowing nodes (`window.tumble`, `window.slide`) ship in
/// Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Window {
    /// Inclusive UTC start instant.
    pub start: DateTime<Utc>,
    /// Exclusive UTC end instant.
    pub end: DateTime<Utc>,
}

impl Window {
    /// Construct a [`Window`].
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// A degenerate "point-in-time" window — `start == end`.
    /// Phase 1 IoT rules emit this on every verdict.
    pub fn instant(at: DateTime<Utc>) -> Self {
        Self { start: at, end: at }
    }
}
