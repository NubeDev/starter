//! The detection scheduler: a single background task that runs due detections.
//!
//! A near-copy of [`crate::alerting::schedule`] — same `TICK`, same
//! `claim_due` + `FOR UPDATE SKIP LOCKED` claim (here over `nexus_detections`),
//! same per-detection error swallowing so one bad detection never stalls the
//! loop, same `run_once` seam for deterministic tests. The body differs only in
//! what each claimed item does: it runs the insight over a query frame and
//! upserts findings, instead of reducing to a scalar and comparing.

use std::time::Duration;

use crate::state::AppState;
use nexus_store::detection::claim_due;

/// How often the scheduler wakes to look for due detections. Each detection has
/// its own interval; this is the polling granularity.
const TICK: Duration = Duration::from_secs(10);

/// Max detections claimed per tick — a backstop so one tick cannot fan out
/// unbounded work; the rest are picked up next tick.
const BATCH: i32 = 100;

/// Spawn the scheduler. Returns the task's `JoinHandle` so the caller can wrap
/// it in the task watchdog (WS-16); the loop runs in the background.
pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            if let Err(e) = run_once(&state).await {
                tracing::warn!(error = %e, "detection scheduler tick failed");
            }
        }
    })
}

/// One scheduler pass: claim due detections and run each. Exposed for tests that
/// drive a single deterministic pass instead of waiting on the timer.
pub async fn run_once(state: &AppState) -> Result<(), String> {
    let due = claim_due(&state.metadata, BATCH)
        .await
        .map_err(|e| e.to_string())?;
    let ctx = super::run::RunContext {
        state,
        metadata: &state.metadata,
        envelope: &state.envelope,
        pools: &state.datasource_pools,
        dev_pool: &state.datasource,
        guards: state.guards,
    };
    for det in due {
        super::run::run_detection(&ctx, &det.tenant_id, det.id).await;
    }
    Ok(())
}
