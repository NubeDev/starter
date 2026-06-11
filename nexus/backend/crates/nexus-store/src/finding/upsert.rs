//! The runner's write path: open/update findings for flagged rows, and
//! auto-resolve findings whose target stopped flagging.
//!
//! Both run inside one tenant transaction per detection run, so a detection's
//! findings move atomically: the flagged set is upserted and everything else for
//! that detection is resolved in the same RLS-bound transaction.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use serde_json::Value;

use super::record::{FindingTransition, NewFinding, Reconciled};
use crate::tenant_tx;

/// Reconcile one detection run's flagged rows against its open findings, all in
/// one tenant transaction:
///
/// 1. Upsert each flagged row by `(detection_id, dedup_key)` against the
///    non-resolved partial unique index — a target flagged for N consecutive
///    intervals stays ONE open finding (its `value`/`at`/`context` updated),
///    not N.
/// 2. Auto-resolve every open/acknowledged finding for the detection whose
///    `dedup_key` is *not* in this run's flagged set — the target stopped being
///    flagged, mirroring the alert "resolved" transition.
///
/// Returns the [`Reconciled`] set — which findings newly opened and which were
/// auto-resolved this run — so the runner can fan those transitions out as
/// notifications. The counts for the run log are just the vec lengths.
pub async fn reconcile(
    pool: &PgPool,
    tenant_id: &str,
    detection_id: Uuid,
    flagged: &[NewFinding],
) -> Result<Reconciled, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;

    let mut opened = Vec::new();
    for f in flagged {
        // ON CONFLICT targets the partial unique index over non-resolved rows.
        // A clash means this target already has an open finding → update it in
        // place; otherwise a new open finding is inserted. `(xmax = 0)` is true
        // only for a genuine INSERT, so it distinguishes a *new* open finding (a
        // transition to notify on) from an update of one already open (no
        // transition). A target whose previous finding was resolved re-inserts a
        // fresh row, so a re-flare correctly counts as a new opening.
        let row = sqlx::query(
            "INSERT INTO nexus_findings \
               (tenant_id, detection_id, at, target, value, context, dedup_key) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (detection_id, dedup_key) WHERE status <> 'resolved' \
             DO UPDATE SET at = EXCLUDED.at, value = EXCLUDED.value, \
                           context = EXCLUDED.context, updated_at = now() \
             RETURNING id, target, value, context, (xmax = 0) AS inserted",
        )
        .bind(tenant_id)
        .bind(detection_id)
        .bind(f.at)
        .bind(&f.target)
        .bind(f.value)
        .bind(&f.context)
        .bind(&f.dedup_key)
        .fetch_one(&mut *tx)
        .await
        .map_err(internal)?;
        if row.get::<bool, _>("inserted") {
            opened.push(row_to_transition(&row));
        }
    }

    // Auto-resolve: any non-resolved finding for this detection whose target was
    // not flagged this run. An empty flagged set resolves them all (the
    // condition cleared everywhere). `<> ALL` over an empty array is true, so
    // the empty-set case needs no special handling. RETURNING surfaces each
    // resolved finding so the runner can notify a "resolved" transition.
    let keys: Vec<String> = flagged.iter().map(|f| f.dedup_key.clone()).collect();
    let resolved_rows = sqlx::query(
        "UPDATE nexus_findings \
         SET status = 'resolved', resolved_at = now(), updated_at = now() \
         WHERE detection_id = $1 AND status <> 'resolved' AND dedup_key <> ALL($2) \
         RETURNING id, target, value, context",
    )
    .bind(detection_id)
    .bind(&keys)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    let resolved = resolved_rows.iter().map(row_to_transition).collect();

    tx.commit().await.map_err(internal)?;
    Ok(Reconciled { opened, resolved })
}

fn row_to_transition(row: &sqlx::postgres::PgRow) -> FindingTransition {
    FindingTransition {
        finding_id: row.get::<Uuid, _>("id"),
        target: row.get::<Value, _>("target"),
        value: row.get::<Option<f64>, _>("value"),
        context: row.get::<Value, _>("context"),
    }
}

/// The most recent `at` across a detection's open findings — a cheap "is this
/// detection producing findings" probe for tests and the run log.
pub async fn latest_open_at(
    pool: &PgPool,
    tenant_id: &str,
    detection_id: Uuid,
) -> Result<Option<DateTime<Utc>>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT max(at) AS at FROM nexus_findings \
         WHERE detection_id = $1 AND status <> 'resolved'",
    )
    .bind(detection_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.get::<Option<DateTime<Utc>>, _>("at"))
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
