//! Time source for the cache layer.
//!
//! Without a swappable clock, every TTL test is either flaky (real
//! sleeps) or slow (real sleeps). The v0 proposal's test story
//! explicitly requires a `MockClock`.

use std::sync::Arc;
use std::time::{Duration, Instant};

/// Source of monotonic timestamps for the cache layer.
pub trait Clock: Send + Sync + 'static {
    /// Now, as a monotonic [`Instant`].
    fn now(&self) -> Instant;
}

/// Process wall clock (`Instant::now`).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Test clock with manual advancement. Use in tests to deterministically
/// step TTL boundaries.
#[derive(Debug, Clone)]
pub struct MockClock {
    inner: Arc<std::sync::Mutex<Instant>>,
}

impl MockClock {
    /// Start at `Instant::now()`.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    /// Advance time by `delta`.
    pub fn advance(&self, delta: Duration) {
        let mut g = self.inner.lock().expect("MockClock poisoned");
        *g += delta;
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        *self.inner.lock().expect("MockClock poisoned")
    }
}
