//! The findings read + manual-lifecycle path used by the API: list with
//! filters, fetch one, acknowledge, and manual resolve.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, QueryBuilder, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{FindingFilter, FindingRecord};
use crate::tenant_tx;

const COLS: &str = "id, tenant_id, detection_id, at, target, value, context, status, \
     acked_by, acked_at, resolved_at, note, dedup_key, created_at, updated_at";

/// List the tenant's findings under `filter`, newest first. The filters compose
/// (all are AND-ed); `target_contains` uses jsonb `@>` containment so a caller
/// can filter by site/meter without knowing the full target shape.
pub async fn list(
    pool: &PgPool,
    tenant_id: &str,
    filter: &FindingFilter,
) -> Result<Vec<FindingRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT ");
    qb.push(COLS).push(" FROM nexus_findings WHERE true");
    if let Some(d) = filter.detection_id {
        qb.push(" AND detection_id = ").push_bind(d);
    }
    if let Some(s) = &filter.status {
        qb.push(" AND status = ").push_bind(s.clone());
    }
    if let Some(t) = &filter.target_contains {
        qb.push(" AND target @> ").push_bind(t.clone());
    }
    if let Some(since) = filter.since {
        qb.push(" AND at >= ").push_bind(since);
    }
    if let Some(until) = filter.until {
        qb.push(" AND at <= ").push_bind(until);
    }
    qb.push(" ORDER BY at DESC LIMIT ").push_bind(filter.limit);

    let rows = qb.build().fetch_all(&mut *tx).await.map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one finding by id within the tenant.
pub async fn get(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<FindingRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(&format!("SELECT {COLS} FROM nexus_findings WHERE id = $1"))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

/// Acknowledge an open finding: `open → acknowledged`, stamping `acked_by`/
/// `acked_at` and an optional note. A no-op (returns `false`) if the finding is
/// already resolved — you cannot ack a closed finding.
pub async fn acknowledge(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    acked_by: &str,
    note: Option<&str>,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query(
        "UPDATE nexus_findings \
         SET status = 'acknowledged', acked_by = $2, acked_at = now(), \
             note = COALESCE($3, note), updated_at = now() \
         WHERE id = $1 AND status <> 'resolved'",
    )
    .bind(id)
    .bind(acked_by)
    .bind(note)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Manually resolve a finding: `* → resolved`, stamping `resolved_at`. Returns
/// `false` if already resolved (nothing changed).
pub async fn resolve(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    note: Option<&str>,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query(
        "UPDATE nexus_findings \
         SET status = 'resolved', resolved_at = now(), \
             note = COALESCE($2, note), updated_at = now() \
         WHERE id = $1 AND status <> 'resolved'",
    )
    .bind(id)
    .bind(note)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> FindingRecord {
    FindingRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        detection_id: row.get::<Uuid, _>("detection_id"),
        at: row.get::<DateTime<Utc>, _>("at"),
        target: row.get::<Value, _>("target"),
        value: row.get::<Option<f64>, _>("value"),
        context: row.get::<Value, _>("context"),
        status: row.get::<String, _>("status"),
        acked_by: row.get::<Option<String>, _>("acked_by"),
        acked_at: row.get::<Option<DateTime<Utc>>, _>("acked_at"),
        resolved_at: row.get::<Option<DateTime<Utc>>, _>("resolved_at"),
        note: row.get::<Option<String>, _>("note"),
        dedup_key: row.get::<String, _>("dedup_key"),
        created_at: row.get::<DateTime<Utc>, _>("created_at"),
        updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
