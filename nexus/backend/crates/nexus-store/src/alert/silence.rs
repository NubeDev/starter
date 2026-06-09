//! Silence (maintenance-window) persistence and the active-silence check.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewSilence, SilenceRecord};
use crate::tenant_tx;

/// Insert a silence.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewSilence,
) -> Result<SilenceRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_alert_silences (tenant_id, rule_id, starts_at, ends_at, reason, created_by) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(new.rule_id)
    .bind(new.starts_at)
    .bind(new.ends_at)
    .bind(&new.reason)
    .bind(&new.created_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(SilenceRecord {
        id: row.get::<Uuid, _>("id"),
        rule_id: new.rule_id,
        starts_at: new.starts_at,
        ends_at: new.ends_at,
        reason: new.reason.clone(),
    })
}

/// List the tenant's silences, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<SilenceRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, rule_id, starts_at, ends_at, reason FROM nexus_alert_silences ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_silence).collect())
}

/// Whether an active silence currently covers `rule_id` — either rule-specific
/// or tenant-wide (`rule_id IS NULL`) — at `now`. The evaluator checks this
/// before notifying.
pub async fn is_silenced(
    pool: &PgPool,
    tenant_id: &str,
    rule_id: Uuid,
    now: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT 1 FROM nexus_alert_silences \
         WHERE (rule_id = $1 OR rule_id IS NULL) AND starts_at <= $2 AND ends_at > $2 LIMIT 1",
    )
    .bind(rule_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.is_some())
}

/// Delete a silence. Returns whether a row matched.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_alert_silences WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn row_to_silence(row: &sqlx::postgres::PgRow) -> SilenceRecord {
    SilenceRecord {
        id: row.get::<Uuid, _>("id"),
        rule_id: row.get::<Option<Uuid>, _>("rule_id"),
        starts_at: row.get::<DateTime<Utc>, _>("starts_at"),
        ends_at: row.get::<DateTime<Utc>, _>("ends_at"),
        reason: row.get::<Option<String>, _>("reason"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
