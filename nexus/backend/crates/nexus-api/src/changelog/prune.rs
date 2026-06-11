//! The retention sweep (WS-12) — a background task that prunes the aged ledger.
//!
//! One task for the process's lifetime, like the alert scheduler. On each tick
//! it deletes ledger rows older than the [`RetentionPolicy`] horizon in capped
//! batches (so one sweep never holds a table-locking delete) until a batch comes
//! back short. The delete crosses tenants through the SECURITY DEFINER
//! `nexus_prune_changes` function — the one controlled cross-tenant write — so
//! the runtime role never needs BYPASSRLS.

use std::time::Duration;

use chrono::Utc;

use crate::changelog::policy::RetentionPolicy;
use crate::state::AppState;

/// How often the sweep wakes. Retention is a slow horizon (days), so an hourly
/// pass keeps the tail bounded without churn; the first pass runs at startup.
const TICK: Duration = Duration::from_secs(3600);

/// Max rows deleted per `nexus_prune_changes` call — the lock-hold backstop. A
/// large backlog drains over several calls within one sweep rather than one
/// long-running delete.
const BATCH: i32 = 5_000;

/// A safety cap on calls per sweep, so a clock skew or pathological backlog can
/// never spin this task. The remainder is picked up on the next tick.
const MAX_CALLS_PER_SWEEP: usize = 1_000;

/// Spawn the retention sweep. Returns the task's `JoinHandle` so the caller can
/// wrap it in the task watchdog (WS-16); the loop runs in the background for the
/// process's lifetime.
pub fn spawn(state: AppState, policy: RetentionPolicy) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            match run_once(&state, policy).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(pruned = n, "audit retention sweep deleted aged rows"),
                Err(e) => tracing::warn!(error = %e, "audit retention sweep failed"),
            }
        }
    })
}

/// One sweep: delete rows older than the horizon in batches until caught up.
/// Returns the total rows pruned. Exposed for tests that drive a single
/// deterministic pass instead of waiting on the timer.
pub async fn run_once(state: &AppState, policy: RetentionPolicy) -> Result<u64, String> {
    let cutoff = policy.cutoff(Utc::now());
    let mut total = 0u64;
    for _ in 0..MAX_CALLS_PER_SWEEP {
        let deleted = nexus_store::changelog::prune_aged(&state.metadata, cutoff, BATCH)
            .await
            .map_err(|e| e.to_string())?;
        total += deleted;
        if (deleted as i32) < BATCH {
            break;
        }
    }
    Ok(total)
}
