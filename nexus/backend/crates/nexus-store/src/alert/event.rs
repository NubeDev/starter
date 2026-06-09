//! Append-only alert-event history.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{EventRecord, NewEvent};
use crate::tenant_tx;

/// Append a transition event.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewEvent) -> Result<EventRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_alert_events \
         (tenant_id, rule_id, transition, value, silenced, notified, detail) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id, at",
    )
    .bind(tenant_id)
    .bind(new.rule_id)
    .bind(&new.transition)
    .bind(new.value)
    .bind(new.silenced)
    .bind(new.notified)
    .bind(&new.detail)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(EventRecord {
        id: row.get::<Uuid, _>("id"),
        rule_id: new.rule_id,
        at: row.get::<DateTime<Utc>, _>("at"),
        transition: new.transition.clone(),
        value: new.value,
        silenced: new.silenced,
        notified: new.notified,
        detail: new.detail.clone(),
    })
}

/// List the tenant's recent events, newest first, capped at `limit`.
pub async fn list(pool: &PgPool, tenant_id: &str, limit: i64) -> Result<Vec<EventRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, rule_id, at, transition, value, silenced, notified, detail \
         FROM nexus_alert_events ORDER BY at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_event).collect())
}

fn row_to_event(row: &sqlx::postgres::PgRow) -> EventRecord {
    EventRecord {
        id: row.get::<Uuid, _>("id"),
        rule_id: row.get::<Uuid, _>("rule_id"),
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
