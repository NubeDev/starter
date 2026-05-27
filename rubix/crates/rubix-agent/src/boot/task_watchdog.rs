//! Wrap a long-lived `JoinHandle` so its unexpected exit is loud.
//!
//! Almost every background task spawned at boot is supposed to run
//! for the lifetime of the process: the scheduler tick, the runtime
//! canary, the SIGUSR1 metrics listener, the PgListener loop, the
//! undo sweep, the pool-telemetry samplers. The `main.rs` pattern
//! `let _scheduler = ...` leaks the handle into the runtime so
//! shutdown drops the task — fine for the happy path, but it also
//! means that if the task panics, returns Ok early, or is silently
//! aborted, *no one notices*. The next freeze investigation has to
//! infer the death by absence (the last "scheduled flow" log line
//! is older than expected, etc.) — exactly the archaeology the
//! recurrence handover described.
//!
//! [`watch`] wraps a `JoinHandle<()>` in a second spawned task that
//! awaits it and emits one ERROR line if the wrapped task exits.
//! The line carries a stable `label` so the operator can grep for
//! `target=rubix.task_watchdog watcher=scheduler` and immediately
//! see "the scheduler died at HH:MM:SS." The error variant is
//! distinguished:
//!
//!   - `outcome=panicked` — the task panicked. The panic hook in
//!     `main.rs` already logged the location + payload; this line
//!     is the "and the parent supervisor noticed" half.
//!   - `outcome=cancelled` — someone called `.abort()`. Only ever
//!     valid during shutdown; mid-run cancellation is a bug.
//!   - `outcome=returned` — the task returned `()` cleanly. For a
//!     supposed-to-be-infinite loop this is also a bug (a `loop {}`
//!     should never reach a return).
//!
//! Zero cost when the task runs forever: the watcher just parks on
//! the inner `JoinHandle`.

use tokio::task::JoinHandle;
use tracing::{error, info};

/// Wrap `handle` in a watchdog spawn. Returns a `JoinHandle<()>`
/// for the *watcher* — same `let _x = watch(...)` leak pattern as
/// the raw handle it replaces.
pub fn watch(label: &'static str, handle: JoinHandle<()>) -> JoinHandle<()> {
    info!(
        target: "rubix.task_watchdog",
        watcher = label,
        "task watchdog armed",
    );
    tokio::spawn(async move {
        let outcome = handle.await;
        match outcome {
            Ok(()) => error!(
                target: "rubix.task_watchdog",
                watcher = label,
                outcome = "returned",
                "supposedly-eternal task returned cleanly — supervisor noticed",
            ),
            Err(e) if e.is_panic() => error!(
                target: "rubix.task_watchdog",
                watcher = label,
                outcome = "panicked",
                "supposedly-eternal task panicked — supervisor noticed (see rubix.panic for payload)",
            ),
            Err(e) if e.is_cancelled() => error!(
                target: "rubix.task_watchdog",
                watcher = label,
                outcome = "cancelled",
                "supposedly-eternal task was aborted — supervisor noticed",
            ),
            Err(_) => error!(
                target: "rubix.task_watchdog",
                watcher = label,
                outcome = "unknown-join-error",
                "supposedly-eternal task ended with an unrecognised JoinError",
            ),
        }
    })
}
