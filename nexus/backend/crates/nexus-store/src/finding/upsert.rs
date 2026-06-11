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

use super::record::NewFinding;
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
/// Returns `(opened_or_updated, auto_resolved)` counts for the run log.
pub async fn reconcile(
    pool: &PgPool,
    tenant_id: &str,
    detection_id: Uuid,
    flagged: &[NewFinding],
) -> Result<(u64, u64), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;

    let mut upserts = 0u64;
    for f in flagged {
        // ON CONFLICT targets the partial unique index over non-resolved rows.
        // A clash means this target already has an open finding → update it in
        // place; otherwise a new open finding is inserted.
        let done = sqlx::query(
            "INSERT INTO nexus_findings \
               (tenant_id, detection_id, at, target, value, context, dedup_key) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (detection_id, dedup_key) WHERE status <> 'resolved' \
             DO UPDATE SET at = EXCLUDED.at, value = EXCLUDED.value, \
                           context = EXCLUDED.context, updated_at = now()",
        )
        .bind(tenant_id)
        .bind(detection_id)
        .bind(f.at)
        .bind(&f.target)
        .bind(f.value)
        .bind(&f.context)
        .bind(&f.dedup_key)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
        upserts += done.rows_affected();
    }

    // Auto-resolve: any non-resolved finding for this detection whose target was
    // not flagged this run. An empty flagged set resolves them all (the
    // condition cleared everywhere). `<> ALL` over an empty array is true, so
    // the empty-set case needs no special handling.
    let keys: Vec<String> = flagged.iter().map(|f| f.dedup_key.clone()).collect();
    let resolved = sqlx::query(
        "UPDATE nexus_findings \
         SET status = 'resolved', resolved_at = now(), updated_at = now() \
         WHERE detection_id = $1 AND status <> 'resolved' AND dedup_key <> ALL($2)",
    )
    .bind(detection_id)
    .bind(&keys)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;

    tx.commit().await.map_err(internal)?;
    Ok((upserts, resolved.rows_affected()))
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
