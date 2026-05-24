//! `Clock` — the wall-clock seam the durable scheduler reads.
//!
//! Phase B.1 (Goal 6, see
//! `.codeless/jobs/rubix-goal-6-weekly-report/SCOPE.md`) introduces
//! [`service::FlowAsService`](crate::service::FlowAsService), the
//! cron-aware companion to [`crate::FlowAsTool`]. Every place that
//! service reads "now" goes through this trait so unit tests can
//! advance time deterministically without sleeping the test runner.
//!
//! - [`SystemClock`] — production path; delegates to
//!   `chrono::Utc::now()`.
//! - [`TestClock`] — a hand-driven clock seeded with a fixed
//!   [`DateTime<Utc>`]. The `advance` / `set` helpers move it
//!   forward; the value is held under a `Mutex` so a
//!   `tokio::spawn`'d worker can both read and mutate it.
//!
//! The trait stays deliberately tiny: one method (`now`) with no
//! `&mut self` requirement and no `async` — the durable scheduler
//! reads the clock on every tick and inside the SQL row-claim
//! loop, where an `async` hop would buy nothing and an `&mut`
//! receiver would force the surface into a `Mutex<Arc<Clock>>`
//! pattern at every call-site.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

/// The wall-clock seam read by
/// [`service::FlowAsService`](crate::service::FlowAsService).
///
/// Implementors return the current UTC wall-clock time. The trait
/// is `Send + Sync` so an `Arc<dyn Clock>` is the natural shared
/// handle — every per-service worker takes one clone of the same
/// `Arc` at boot.
pub trait Clock: Send + Sync + 'static {
    /// Current wall-clock time in UTC.
    fn now(&self) -> DateTime<Utc>;
}

/// Production [`Clock`] — delegates to [`chrono::Utc::now`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl SystemClock {
    /// Construct a fresh [`SystemClock`].
    pub const fn new() -> Self {
        Self
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Hand-driven [`Clock`] for unit and integration tests.
///
/// The held instant lives behind a [`Mutex`] so worker tasks may
/// concurrently read and tests may mutate without an `&mut`
/// receiver on the trait. Mutex contention is irrelevant in
/// tests (single-threaded by convention) and the worker only
/// holds the lock for the duration of a [`Clock::now`] read.
#[derive(Debug, Clone)]
pub struct TestClock {
    inner: Arc<Mutex<DateTime<Utc>>>,
}

impl TestClock {
    /// Construct a clock pinned at `seed`.
    pub fn new(seed: DateTime<Utc>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(seed)),
        }
    }

    /// Construct a clock pinned at the Unix epoch — the default
    /// any test that doesn't care about the absolute date can
    /// use.
    pub fn epoch() -> Self {
        Self::new(DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is representable"))
    }

    /// Replace the held instant outright.
    pub fn set(&self, when: DateTime<Utc>) {
        *self.inner.lock().unwrap_or_else(|e| e.into_inner()) = when;
    }

    /// Advance the held instant by `delta`.
    pub fn advance(&self, delta: Duration) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard += delta;
    }
}

impl Clock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        *self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}
