//! Wrap a long-lived `JoinHandle` so its unexpected exit is loud.
//!
//! Almost every background task spawned at boot is supposed to run for the
//! lifetime of the process: the alert scheduler, the detection scheduler, the
//! audit-retention sweep, the runtime canary. The `main.rs` pattern
//! `let _x = ...::spawn(state)` leaks the handle into the runtime so shutdown
//! drops the task — fine for the happy path, but it also means that if the task
//! panics, returns `Ok` early, or is silently aborted, *no one notices*. The
//! next freeze investigation has to infer the death by absence (the last
//! "evaluated due rules" log line is older than the tick interval, etc.) —
//! exactly the archaeology this module eliminates.
//!
//! [`watch`] wraps a `JoinHandle<()>` in a second spawned task that awaits it
//! and emits one ERROR line if the wrapped task exits. The line carries a stable
//! `watcher` label so the operator can grep for
//! `target=nexus.task_watchdog watcher=alert_scheduler` and immediately see "the
//! scheduler died at HH:MM:SS." The `outcome` is distinguished:
//!
//!   - `outcome=returned` — the task returned `()` cleanly. For a
//!     supposed-to-be-infinite `loop {}` this is a bug.
//!   - `outcome=panicked` — the task panicked. The panic hook already logged the
//!     location + payload; this line is the "and the supervisor noticed" half.
//!   - `outcome=cancelled` — someone called `.abort()`. Only ever valid during
//!     shutdown; mid-run cancellation is a bug.
//!   - `outcome=unknown-join-error` — an unrecognised `JoinError` variant.
//!
//! Zero cost when the task runs forever: the watcher just parks on the inner
//! `JoinHandle`.
//!
//! Ported from `rubix-agent/src/boot/task_watchdog.rs`.

use tokio::task::JoinHandle;
use tracing::{error, info};

/// Wrap `handle` in a watchdog spawn. Returns a `JoinHandle<()>` for the
/// *watcher* — same `let _x = watch(...)` leak pattern as the raw handle it
/// replaces, so caller lifetime semantics are unchanged.
pub fn watch(label: &'static str, handle: JoinHandle<()>) -> JoinHandle<()> {
    info!(
        target: "nexus.task_watchdog",
        watcher = label,
        "task watchdog armed",
    );
    tokio::spawn(async move {
        match handle.await {
            Ok(()) => error!(
                target: "nexus.task_watchdog",
                watcher = label,
                outcome = "returned",
                "supposedly-eternal task returned cleanly — supervisor noticed",
            ),
            Err(e) if e.is_panic() => error!(
                target: "nexus.task_watchdog",
                watcher = label,
                outcome = "panicked",
                "supposedly-eternal task panicked — supervisor noticed (see panic log for payload)",
            ),
            Err(e) if e.is_cancelled() => error!(
                target: "nexus.task_watchdog",
                watcher = label,
                outcome = "cancelled",
                "supposedly-eternal task was aborted — supervisor noticed",
            ),
            Err(_) => error!(
                target: "nexus.task_watchdog",
                watcher = label,
                outcome = "unknown-join-error",
                "supposedly-eternal task ended with an unrecognised JoinError",
            ),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn watcher_completes_after_clean_return() {
        // A task that returns immediately — the watcher should observe the
        // `returned` outcome and itself complete.
        let inner = tokio::spawn(async {});
        let watcher = watch("test_clean_return", inner);
        watcher.await.expect("watcher should not panic");
    }

    #[tokio::test]
    async fn watcher_survives_a_panicking_task() {
        // A task that panics must not bring the watcher down: the watcher
        // catches the join error and logs it, then completes normally.
        let inner = tokio::spawn(async { panic!("boom") });
        let watcher = watch("test_panic", inner);
        watcher.await.expect("watcher itself must not panic");
    }
}
