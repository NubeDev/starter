//! On-demand tokio runtime metrics dump.
//!
//! Listens for `SIGUSR1` and emits a single tracing line with the
//! state of the current tokio runtime. Pairs with
//! [`crate::boot::runtime_canary`] for freeze investigations: the
//! canary tells you *that* the runtime is wedged (or not); this
//! dump tells you *how* — how many workers exist, how many tasks
//! are alive, how many blocking threads are in flight.
//!
//! ## How to use during a freeze
//!
//! ```text
//! pid=$(pgrep -f 'rubix-agent$')
//! kill -USR1 $pid
//! tail -n 5 /tmp/rubix-agent.log | grep rubix.runtime_metrics
//! ```
//!
//! ## Why SIGUSR1 (and not a HTTP route or tokio-console)
//!
//! - A HTTP route can't run if the runtime is wedged — that's
//!   exactly the case we need to diagnose.
//! - `tokio-console` requires `--cfg tokio_unstable` and a separate
//!   crate. Worth adding later if this dump's signal turns out to
//!   be insufficient, but the rebuild + dep-tree cost is real.
//! - Unix signals are delivered to the *process*, not a tokio task,
//!   so the handler runs even when every worker is parked. The
//!   signal-listener task itself can park, but the kernel still
//!   delivers the signal and `tokio::signal::unix::Signal` polls it
//!   the moment the runtime makes any forward progress at all —
//!   which is the discriminator we want.
//!
//! ## What you get without `--cfg tokio_unstable`
//!
//! `RuntimeMetrics` exposes only `num_workers()` and
//! `num_alive_tasks()` on stable. That's enough to answer:
//!
//!   - Are workers still alive? (`num_workers` matches config)
//!   - Are tasks leaking or stuck? (`num_alive_tasks` rising across
//!     dumps ⇒ leak; flat across dumps while symptoms frozen ⇒
//!     workers parked on a lock or socket)
//!
//! Blocking-pool saturation (`num_blocking_threads`), per-task
//! names, parking sites, and busy ratios all require
//! `--cfg tokio_unstable` plus `console-subscriber`. Add that only
//! if this dump leaves the next freeze ambiguous.

use tokio::task::JoinHandle;
use tracing::{error, info};

/// Spawn the SIGUSR1 listener. Returns the handle the caller leaks
/// into the process lifetime (mirrors `runtime_canary::spawn`).
///
/// Fails only if `signal(SIGUSR1)` cannot be installed — that means
/// some other component already grabbed the signal, which is worth
/// surfacing rather than silently no-op'ing.
pub fn spawn() -> std::io::Result<JoinHandle<()>> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigusr1 = signal(SignalKind::user_defined1())?;
    info!(
        target: "rubix.boot.runtime_metrics",
        "SIGUSR1 handler installed — kill -USR1 <pid> to dump runtime metrics",
    );

    let handle = tokio::spawn(async move {
        loop {
            if sigusr1.recv().await.is_none() {
                // The signal stream closed — unrecoverable. Log
                // once and exit so we don't spin.
                error!(
                    target: "rubix.boot.runtime_metrics",
                    "SIGUSR1 signal stream closed — metrics dumps disabled",
                );
                return;
            }
            dump();
        }
    });
    Ok(handle)
}

/// Emit one metrics line. Pulled out of `spawn` so a future caller
/// (e.g. a test, or a periodic ticker) can invoke it directly.
fn dump() {
    let handle = tokio::runtime::Handle::current();
    let metrics = handle.metrics();
    info!(
        target: "rubix.runtime_metrics",
        num_workers = metrics.num_workers(),
        num_alive_tasks = metrics.num_alive_tasks(),
        "tokio runtime metrics snapshot",
    );
}
