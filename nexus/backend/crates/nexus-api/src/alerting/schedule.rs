//! The alert scheduler: a single background task that evaluates due rules.
//!
//! On each tick it claims the rules whose `next_eval_at` has passed (the claim
//! advances their next time, so a rule is taken once per interval) and evaluates
//! each under its tenant. Single-node for v1, like the FlowManager — a
//! multi-node deploy needs leader election or a shared queue so a rule is
//! evaluated once across replicas; the `FOR UPDATE SKIP LOCKED` claim already
//! makes that upgrade safe. Spawned once at startup; it runs for the process's
//! lifetime.

use std::time::Duration;

use crate::state::AppState;
use nexus_store::alert::claim_due;

/// How often the scheduler wakes to look for due rules. Rules have their own
/// per-rule interval; this is just the polling granularity.
const TICK: Duration = Duration::from_secs(10);

/// Max rules claimed per tick — a backstop so one tick cannot fan out unbounded
/// evaluation work; the rest are picked up on the next tick.
const BATCH: i32 = 100;

/// Spawn the scheduler. Returns immediately; the loop runs in the background.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            if let Err(e) = run_once(&state).await {
                tracing::warn!(error = %e, "alert scheduler tick failed");
            }
        }
    });
}

/// One scheduler pass: claim due rules and evaluate each. Exposed for tests that
/// drive a single deterministic pass instead of waiting on the timer.
pub async fn run_once(state: &AppState) -> Result<(), String> {
    let due = claim_due(&state.metadata, BATCH)
        .await
        .map_err(|e| e.to_string())?;
    let ctx = super::evaluate::EvalContext {
        metadata: &state.metadata,
        envelope: &state.envelope,
        pools: &state.datasource_pools,
        dev_pool: &state.datasource,
        guards: state.guards,
    };
    for rule in due {
        super::evaluate::evaluate_rule(&ctx, &rule.tenant_id, rule.id).await;
    }
    Ok(())
}
