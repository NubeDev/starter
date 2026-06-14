//! Notification-channel persistence.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{ChannelRecord, NewChannel};
use crate::tenant_tx;

/// Insert a channel. A duplicate name in the tenant is a `Conflict`.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewChannel,
) -> Result<ChannelRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_notify_channels (tenant_id, name, kind, config) \
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.kind)
    .bind(&new.config)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(ChannelRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        kind: new.kind.clone(),
        config: new.config.clone(),
    })
}

/// List the tenant's channels, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<ChannelRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, kind, config FROM nexus_notify_channels ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_channel).collect())
}

/// Fetch the channels with the given ids within the tenant — the runner's lookup
/// when fanning a finding transition out to a detection's targets.
pub async fn by_ids(
    pool: &PgPool,
    tenant_id: &str,
    ids: &[Uuid],
) -> Result<Vec<ChannelRecord>, Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, kind, config FROM nexus_notify_channels WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_channel).collect())
}

/// Delete a channel. Returns whether a row matched.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_notify_channels WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn row_to_channel(row: &sqlx::postgres::PgRow) -> ChannelRecord {
    ChannelRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        kind: row.get::<String, _>("kind"),
        config: row.get::<serde_json::Value, _>("config"),
    }
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a notification channel with that name already exists".into(),
            };
        }
    }
    internal(e)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
