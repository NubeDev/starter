//! Tokio runtime liveness canary.
//!
//! Spawns a background task that bumps a shared `AtomicU64` to the current
//! unix-epoch-seconds **once per second**. Reading the atomic from a request
//! handler lets the operator distinguish three failure modes that look identical
//! from the outside (all three present as "the server stopped responding"):
//!
//! 1. **Runtime wedge.** Every tokio worker thread is parked on a futex; the
//!    canary atomic has not advanced. Cause is in runtime-internals territory (a
//!    sync `Mutex` held across an `.await`, a `tracing` subscriber that blocks
//!    every log call, a panic in a custom waker, ...). `/livez` returns 503 with
//!    the staleness in its body.
//! 2. **HTTP layer wedge.** The runtime is alive (atomic advancing) but the
//!    `axum::serve` accept loop or a middleware layer is stuck. `/livez` returns
//!    200 but external HTTP probes still time out — operator sees runtime alive +
//!    listener dead and looks at the tower/middleware layers.
//! 3. **Application wedge.** Both `/livez` and `/health` respond fine; the
//!    symptom is one slow handler. `/readyz` may additionally flag a DB issue.
//!    Standard handler-level debugging.
//!
//! The freshness threshold ([`STALENESS_BUDGET`]) is intentionally generous (5s)
//! so a transient stop-the-world (GC-equivalent tokio pause) does not trip it.
//!
//! Ported from `rubix-agent/src/boot/runtime_canary.rs`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::task::JoinHandle;
use tracing::info;

/// If the canary atomic is older than this, `/livez` returns 503. Picked to be
/// larger than a single tick interval (1s) plus a generous fudge for scheduler
/// hiccups.
pub const STALENESS_BUDGET: Duration = Duration::from_secs(5);

/// Shared handle to the last-tick timestamp. Cloneable so the canary task and
/// the `/livez` handler hold different `Arc`s over the same atomic. Lives in
/// `AppState` so any handler can read it.
#[derive(Clone, Debug, Default)]
pub struct Canary {
    last_tick_unix_secs: Arc<AtomicU64>,
}

impl Canary {
    /// Construct a canary whose atomic starts at "now". A handler that reads it
    /// immediately after construction sees a fresh timestamp rather than 0,
    /// which would otherwise look like a permanent wedge before the first tick
    /// lands.
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            last_tick_unix_secs: Arc::new(AtomicU64::new(now)),
        }
    }

    /// Seconds since the last canary bump. Returns `None` only on the clock-skew
    /// path where the atomic holds a future timestamp.
    pub fn staleness(&self) -> Option<Duration> {
        let last = self.last_tick_unix_secs.load(Ordering::Relaxed);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        Some(Duration::from_secs(now.saturating_sub(last)))
    }

    /// Whether the runtime looks wedged: the atomic is stale past
    /// [`STALENESS_BUDGET`]. A clock-skew read (`staleness() == None`) is treated
    /// as *not* stale — the runtime is plainly ticking if it just stored a
    /// future-looking timestamp.
    pub fn is_stale(&self) -> bool {
        self.staleness().is_some_and(|s| s > STALENESS_BUDGET)
    }
}

/// Spawn the canary tick task. Returns the [`Canary`] handle the HTTP route uses
/// to read the atomic, plus the `JoinHandle` the caller wraps in the task
/// watchdog and leaks into the process lifetime.
pub fn spawn() -> (Canary, JoinHandle<()>) {
    let canary = Canary::new();
    let cloned = canary.clone();
    info!(
        target: "nexus.boot.runtime_canary",
        tick_interval_seconds = 1u64,
        staleness_budget_seconds = STALENESS_BUDGET.as_secs(),
        "runtime liveness canary started",
    );
    let handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        // `Burst` is the default missed-tick behaviour — fine here: if we ever
        // lag, we want the canary to tick as fast as it can to catch up so
        // `/livez` recovers immediately once the runtime un-wedges.
        //
        // Every HEARTBEAT_EVERY ticks the loop also emits one INFO line. The
        // point of *this* line is that it is emitted from the canary task
        // itself, so its absence is unambiguous evidence the canary loop is
        // parked (rather than the log writer being blocked or the global
        // subscriber wedging) — the next freeze investigation can compare the
        // canary heartbeat cadence against other periodic logs to localise the
        // wedge.
        const HEARTBEAT_EVERY: u64 = 60;
        let mut tick_count: u64 = 0;
        loop {
            ticker.tick().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            cloned.last_tick_unix_secs.store(now, Ordering::Relaxed);
            tick_count = tick_count.wrapping_add(1);
            if tick_count % HEARTBEAT_EVERY == 0 {
                info!(
                    target: "nexus.runtime_canary",
                    tick_count = tick_count,
                    "canary heartbeat",
                );
            }
        }
    });
    (canary, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_canary_is_not_stale() {
        let canary = Canary::new();
        assert!(!canary.is_stale());
        assert!(canary.staleness().unwrap() < STALENESS_BUDGET);
    }

    #[tokio::test(start_paused = true)]
    async fn ticking_keeps_canary_fresh() {
        let (canary, _h) = spawn();
        // Advance virtual time past the staleness budget; with the tick task
        // running, each second bumps the atomic so it never goes stale.
        for _ in 0..(STALENESS_BUDGET.as_secs() + 2) {
            tokio::time::advance(Duration::from_secs(1)).await;
            tokio::task::yield_now().await;
        }
        assert!(!canary.is_stale());
    }
}
