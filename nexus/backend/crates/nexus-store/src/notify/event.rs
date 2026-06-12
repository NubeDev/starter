//! Notify-event persistence: the append-only history of notifications a
//! detection delivered (or suppressed) on a finding transition.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewNotifyEvent, NotifyEventRecord};
use crate::tenant_tx;

/// Append a notify event.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewNotifyEvent) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_notify_events \
           (tenant_id, detection_id, finding_id, transition, value, silenced, notified, detail) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(tenant_id)
    .bind(new.detection_id)
    .bind(new.finding_id)
    .bind(&new.transition)
    .bind(new.value)
    .bind(new.silenced)
    .bind(new.notified)
    .bind(&new.detail)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(())
}

/// List the tenant's most recent notify events, newest first, up to `limit`.
pub async fn list(
    pool: &PgPool,
    tenant_id: &str,
    limit: i64,
) -> Result<Vec<NotifyEventRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, detection_id, finding_id, at, transition, value, silenced, notified, detail \
         FROM nexus_notify_events ORDER BY at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_event).collect())
}

fn row_to_event(row: &sqlx::postgres::PgRow) -> NotifyEventRecord {
    NotifyEventRecord {
        id: row.get::<Uuid, _>("id"),
        detection_id: row.get::<Uuid, _>("detection_id"),
        finding_id: row.get::<Option<Uuid>, _>("finding_id"),
        at: row.get::<DateTime<Utc>, _>("at"),
        transition: row.get::<String, _>("transition"),
        value: row.get::<Option<f64>, _>("value"),
        silenced: row.get::<bool, _>("silenced"),
        notified: row.get::<bool, _>("notified"),
        detail: row.get::<Option<String>, _>("detail"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
