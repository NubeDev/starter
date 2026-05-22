//! [`PgRunStore`] — run lifecycle, per-tick checkpoints,
//! dedup lookup (D-F3.3, D-F3.8, D-F3.9, D-F3.12). Postgres twin of
//! `SqliteRunStore`.

use async_trait::async_trait;
use sqlx::Row;
use starter_flow_spi::flow::{
    CheckpointRetention, DedupKey, FlowError, FlowResult, FlowRevisionId, RunCheckpoint, RunId,
    RunOpts, RunOutcome, RunState, RunStore,
};
use starter_flow_spi::node::{SlotRef, SlotValue};
use starter_flow_spi::Principal;

use super::schema::{from_value, sqlx_backend, to_value};
use crate::pool::Pool;

/// Postgres-backed [`RunStore`].
#[derive(Clone)]
pub struct PgRunStore {
    pool: Pool,
}

impl PgRunStore {
    /// Construct a [`PgRunStore`] over an existing [`Pool`].
    /// Pair with [`super::FLOW_MIGRATION_SOURCE`] on the migrate
    /// chain.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

/// Serialize the slot-write batch for the JSONB checkpoint row.
/// Kept out of band of `schema::to_value` so the borrow stays
/// explicit (the SPI passes the batch by slice, not by owned
/// `Vec`).
fn writes_to_value(writes: &[(SlotRef, SlotValue)]) -> FlowResult<serde_json::Value> {
    serde_json::to_value(writes).map_err(|e| FlowError::Backend(format!("serialize writes: {e}")))
}

/// `RunState` snake-case discriminant for the `runs.status` column.
fn status_str(state: RunState) -> &'static str {
    // `RunState` is `#[non_exhaustive]` per the SPI so future
    // additive variants compile; the wildcard maps the unknown
    // variant to "running" defensively (the engine never feeds
    // an unknown variant through this path, but a `Backend`
    // surface here would mask the actual checkpoint write).
    match state {
        RunState::Running => "running",
        RunState::Paused => "paused",
        RunState::Completed => "completed",
        RunState::Failed => "failed",
        RunState::Cancelled => "cancelled",
        _ => "running",
    }
}

/// `RunOutcome` snake-case discriminant for the `runs.status`
/// column on finish.
fn outcome_status(outcome: &RunOutcome) -> &'static str {
    // `RunOutcome` is `#[non_exhaustive]`; unknown variants are
    // treated as "failed" so an operator scanning `runs.status`
    // can spot a binary/data skew.
    match outcome {
        RunOutcome::Completed { .. } => "completed",
        RunOutcome::Failed { .. } => "failed",
        RunOutcome::Cancelled => "cancelled",
        _ => "failed",
    }
}

#[async_trait]
impl RunStore for PgRunStore {
    async fn start(
        &self,
        run_id: RunId,
        flow_revision: FlowRevisionId,
        opts: RunOpts,
        principal: Principal,
        dedup: Option<DedupKey>,
    ) -> FlowResult<()> {
        let pool = self.pool.sqlx();
        let opts_value = to_value(&opts)?;
        let principal_value = to_value(&principal)?;
        let (service_name, dedup_key) = match dedup {
            Some(d) => (Some(d.service_name), Some(d.key)),
            None => (None, None),
        };

        // Single INSERT — the UNIQUE partial index on
        // (service_name, dedup_key) is the D-F3.12 race-safety
        // backstop. Collisions surface as a `Backend` error so
        // `FlowAsService` can re-query `find_by_dedup_key` and
        // short-circuit to the prior run.
        sqlx::query(
            "INSERT INTO runs \
             (run_id, flow_revision_id, principal_json, run_opts_json, \
              status, dedup_key, service_name) \
             VALUES ($1, $2, $3, $4, 'running', $5, $6)",
        )
        .bind(run_id.0.to_string())
        .bind(flow_revision.0.to_string())
        .bind(&principal_value)
        .bind(&opts_value)
        .bind(&dedup_key)
        .bind(&service_name)
        .execute(pool)
        .await
        .map_err(sqlx_backend)?;
        Ok(())
    }

    async fn checkpoint(
        &self,
        run_id: RunId,
        seq: u64,
        state: RunState,
        writes: &[(SlotRef, SlotValue)],
    ) -> FlowResult<()> {
        let pool = self.pool.sqlx();
        let writes_value = writes_to_value(writes)?;
        let state_value = to_value(&state)?;
        let run_id_s = run_id.0.to_string();
        // Postgres `BIGINT` is i64; propagator tick is u64. The
        // 2^63 ceiling is a non-issue (one tick per ns for 292
        // years), but cast explicitly so the lossy path is
        // visible.
        let seq_i = seq as i64;

        // Load the run's retention policy once. Reading it inside
        // the transaction would serialize with concurrent
        // checkpoints on other runs; reading outside is safe
        // because `RunOpts` is set at `start` and immutable for
        // the run's lifetime.
        let retention = load_retention(pool, &run_id_s).await?;

        // Transaction so the insert + prune + status update form
        // one atomic write batch (D-F3.8). A crash anywhere in
        // this block leaves either the prior latest checkpoint or
        // this one as the visible state — never a half-pruned
        // intermediate.
        let mut tx = pool.begin().await.map_err(sqlx_backend)?;

        sqlx::query(
            "INSERT INTO run_checkpoints \
             (run_id, seq, run_state_json, slot_writes_json) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&run_id_s)
        .bind(seq_i)
        .bind(&state_value)
        .bind(&writes_value)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_backend)?;

        // Mirror the engine-typed state into runs.status so
        // `list_open` is one indexed lookup.
        sqlx::query("UPDATE runs SET status = $1 WHERE run_id = $2")
            .bind(status_str(state))
            .bind(&run_id_s)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_backend)?;

        // D-F3.9 in-tx pruning. Bounded(n): keep the last n rows
        // (highest seq). Unbounded: skip pruning entirely.
        if let CheckpointRetention::Bounded(n) = retention {
            let keep = n as i64;
            sqlx::query(
                "DELETE FROM run_checkpoints \
                 WHERE run_id = $1 \
                   AND seq NOT IN ( \
                       SELECT seq FROM run_checkpoints \
                       WHERE run_id = $1 \
                       ORDER BY seq DESC \
                       LIMIT $2 \
                   )",
            )
            .bind(&run_id_s)
            .bind(keep)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_backend)?;
        }

        tx.commit().await.map_err(sqlx_backend)?;
        Ok(())
    }

    async fn load(&self, run_id: RunId) -> FlowResult<Option<RunCheckpoint>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query(
            "SELECT seq, run_state_json, slot_writes_json \
             FROM run_checkpoints \
             WHERE run_id = $1 \
             ORDER BY seq DESC \
             LIMIT 1",
        )
        .bind(run_id.0.to_string())
        .fetch_optional(pool)
        .await
        .map_err(sqlx_backend)?;

        let Some(row) = row else { return Ok(None) };
        let seq: i64 = row.try_get("seq").map_err(sqlx_backend)?;
        let state_value: serde_json::Value = row.try_get("run_state_json").map_err(sqlx_backend)?;
        let writes_value: serde_json::Value =
            row.try_get("slot_writes_json").map_err(sqlx_backend)?;
        let state: RunState = from_value("run_checkpoints.run_state_json", state_value)?;
        let writes: Vec<(SlotRef, SlotValue)> =
            from_value("run_checkpoints.slot_writes_json", writes_value)?;
        Ok(Some(RunCheckpoint::new(run_id, seq as u64, state, writes)))
    }

    async fn finish(&self, run_id: RunId, outcome: RunOutcome) -> FlowResult<()> {
        let pool = self.pool.sqlx();
        let outcome_value = to_value(&outcome)?;
        let status = outcome_status(&outcome);
        let run_id_s = run_id.0.to_string();

        // Atomic with the keep-final-row prune per D-F3.9: drop
        // every checkpoint except the latest, then mark the run
        // finished. A crash anywhere leaves either (a) the run
        // still open with its full checkpoint trail, or (b) the
        // run finished with exactly one trailing checkpoint.
        let mut tx = pool.begin().await.map_err(sqlx_backend)?;

        sqlx::query(
            "DELETE FROM run_checkpoints \
             WHERE run_id = $1 \
               AND seq < (SELECT COALESCE(MAX(seq), 0) FROM run_checkpoints WHERE run_id = $1)",
        )
        .bind(&run_id_s)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_backend)?;

        sqlx::query(
            "UPDATE runs SET \
                 status = $1, \
                 outcome_json = $2, \
                 finished_at = NOW() \
             WHERE run_id = $3",
        )
        .bind(status)
        .bind(&outcome_value)
        .bind(&run_id_s)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_backend)?;

        tx.commit().await.map_err(sqlx_backend)?;
        Ok(())
    }

    async fn list_open(&self) -> FlowResult<Vec<RunId>> {
        let pool = self.pool.sqlx();
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT run_id FROM runs WHERE finished_at IS NULL ORDER BY created_at")
                .fetch_all(pool)
                .await
                .map_err(sqlx_backend)?;
        rows.into_iter()
            .map(|(s,)| {
                uuid::Uuid::parse_str(&s)
                    .map(RunId)
                    .map_err(|e| FlowError::Backend(format!("runs.run_id: {e}")))
            })
            .collect()
    }

    async fn find_by_dedup_key(
        &self,
        service_name: &str,
        dedup_key: &str,
    ) -> FlowResult<Option<RunId>> {
        let pool = self.pool.sqlx();
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT run_id FROM runs \
             WHERE service_name = $1 AND dedup_key = $2 \
             LIMIT 1",
        )
        .bind(service_name)
        .bind(dedup_key)
        .fetch_optional(pool)
        .await
        .map_err(sqlx_backend)?;
        row.map(|(s,)| {
            uuid::Uuid::parse_str(&s)
                .map(RunId)
                .map_err(|e| FlowError::Backend(format!("runs.run_id: {e}")))
        })
        .transpose()
    }
}

/// Fetch the `CheckpointRetention` an open run was started with.
/// Defaults to [`CheckpointRetention::default()`] if the run row
/// is missing (a checkpoint without a `runs` row is itself a
/// caller bug; pruning still runs with sensible defaults rather
/// than silently disabling itself).
async fn load_retention(pool: &sqlx::PgPool, run_id: &str) -> FlowResult<CheckpointRetention> {
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT run_opts_json FROM runs WHERE run_id = $1")
            .bind(run_id)
            .fetch_optional(pool)
            .await
            .map_err(sqlx_backend)?;
    let Some((raw,)) = row else {
        return Ok(CheckpointRetention::default());
    };
    let opts: RunOpts = from_value("runs.run_opts_json", raw)?;
    Ok(opts.checkpoint_retention)
}
