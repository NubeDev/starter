//! Periodic `PgPool` telemetry logger.
//!
//! Tracks `size()`, `num_idle()`, and a derived `in_use` for every
//! pool the agent owns. Logs at INFO every [`INTERVAL`]; logs at
//! WARN whenever `in_use >= max - 1` so pool starvation shows up in
//! the agent log without an operator having to query Postgres
//! `pg_stat_activity` in real time.
//!
//! Wiring (see `main.rs`): one `spawn` call per pool, labelled with
//! a human-readable name so the lines are greppable
//! (`pool=rubix-mcp`, `pool=rubix-flow-notify`, `pool=warehouse`,
//! …). The returned [`JoinHandle`] is leaked into the process
//! lifetime, mirroring the `_undo_sweep` and `_scheduler` pattern
//! in `main.rs`.
//!
//! This is observability-only: it never touches the pool itself and
//! never holds a connection.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Sample cadence. 30s matches the operator's typical "watch the
/// log" rhythm — small enough to catch a burst, large enough to
/// not spam the log file under steady state.
const INTERVAL: Duration = Duration::from_secs(30);

/// Spawn a sampling task that logs pool stats every [`INTERVAL`].
///
/// `label` is rendered as the `pool` field so multiple pools can
/// coexist in one log stream.
pub fn spawn(pool: PgPool, label: &'static str) -> JoinHandle<()> {
    let max = pool.options().get_max_connections() as usize;
    info!(
        target: "rubix.boot.pool_telemetry",
        pool = label,
        max_connections = max,
        sample_interval_seconds = INTERVAL.as_secs(),
        "pool telemetry started",
    );
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);
        // Skip the immediate first tick so the boot log isn't
        // cluttered with one telemetry line per pool at startup —
        // the `info!` above already records that telemetry is on.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let size = pool.size() as usize;
            let idle = pool.num_idle();
            let in_use = size.saturating_sub(idle);
            // The "starvation" predicate fires when we are
            // (a) one connection away from saturation AND
            // (b) the pool is large enough that "one short" is
            //     actually a problem.
            //
            // The `max >= 4` guard exists because tiny pools
            // (the LISTEN listener uses `max=2` with one
            // connection pinned forever by `PgListener`) would
            // otherwise trip the WARN predicate on every single
            // tick — pure noise that drowns out real saturation
            // on the 16-conn pools. For tiny pools we only WARN
            // when literally exhausted.
            let saturated = if max >= 4 {
                in_use >= max.saturating_sub(1)
            } else {
                in_use >= max
            };
            if saturated {
                warn!(
                    target: "rubix.boot.pool_telemetry",
                    pool = label,
                    size,
                    idle,
                    in_use,
                    max,
                    "pool near saturation — investigate pg_stat_activity",
                );
            } else {
                info!(
                    target: "rubix.boot.pool_telemetry",
                    pool = label,
                    size,
                    idle,
                    in_use,
                    max,
                    "pool stats",
                );
            }
        }
    })
}
