//! Detection persistence: create, list, fetch, update, delete — all tenant-scoped.

use serde_json::Value;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DetectionPatch, DetectionRecord, NewDetection};
use crate::tenant_tx;

const COLS: &str = "id, tenant_id, name, insight_id, datasource_id, sql, params, sources, \
     flag_column, target_columns, value_column, for_secs, interval_secs, enabled, \
     channel_ids, message_template";

/// Insert a detection. A duplicate name in the tenant is a `Conflict`; an
/// `insight_id` that does not resolve under the tenant is a foreign-key error
/// surfaced as `Invalid`.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewDetection,
) -> Result<DetectionRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_detections \
         (tenant_id, name, insight_id, datasource_id, sql, params, sources, flag_column, \
          target_columns, value_column, for_secs, interval_secs, enabled, channel_ids, \
          message_template) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(new.insight_id)
    .bind(new.datasource_id)
    .bind(&new.sql)
    .bind(&new.params)
    .bind(&new.sources)
    .bind(&new.flag_column)
    .bind(&new.target_columns)
    .bind(&new.value_column)
    .bind(new.for_secs)
    .bind(new.interval_secs)
    .bind(new.enabled)
    .bind(&new.channel_ids)
    .bind(&new.message_template)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_invalid)?;
    let id = row.get::<Uuid, _>("id");
    tx.commit().await.map_err(internal)?;

    Ok(DetectionRecord {
        id,
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        insight_id: new.insight_id,
        datasource_id: new.datasource_id,
        sql: new.sql.clone(),
        params: new.params.clone(),
        sources: new.sources.clone(),
        flag_column: new.flag_column.clone(),
        target_columns: new.target_columns.clone(),
        value_column: new.value_column.clone(),
        for_secs: new.for_secs,
        interval_secs: new.interval_secs,
        enabled: new.enabled,
        channel_ids: new.channel_ids.clone(),
        message_template: new.message_template.clone(),
    })
}

/// List the tenant's detections, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<DetectionRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(&format!(
        "SELECT {COLS} FROM nexus_detections ORDER BY created_at DESC"
    ))
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one detection by id within the tenant.
pub async fn get(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<DetectionRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(&format!(
        "SELECT {COLS} FROM nexus_detections WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

/// Apply `patch` to detection `id`. Returns whether a row matched.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &DetectionPatch,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query(
        // `datasource_id` can't use COALESCE: COALESCE can't express "set to
        // NULL" (clear → dev pool) distinct from "leave unchanged". `$13` is the
        // touch flag (Some(_) in the patch), `$14` the new value (which may be
        // NULL); when not touched the column keeps its current value.
        "UPDATE nexus_detections SET \
           name           = COALESCE($2, name), \
           insight_id     = COALESCE($3, insight_id), \
           datasource_id  = CASE WHEN $13 THEN $14 ELSE datasource_id END, \
           sql            = COALESCE($4, sql), \
           params         = COALESCE($5, params), \
           sources        = COALESCE($6, sources), \
           flag_column    = COALESCE($7, flag_column), \
           target_columns = COALESCE($8, target_columns), \
           value_column   = COALESCE($9, value_column), \
           for_secs       = COALESCE($10, for_secs), \
           interval_secs  = COALESCE($11, interval_secs), \
           enabled        = COALESCE($12, enabled), \
           channel_ids    = COALESCE($15, channel_ids), \
           message_template = COALESCE($16, message_template) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(patch.insight_id)
    .bind(&patch.sql)
    .bind(&patch.params)
    .bind(&patch.sources)
    .bind(&patch.flag_column)
    .bind(patch.target_columns.as_deref())
    .bind(&patch.value_column)
    .bind(patch.for_secs)
    .bind(patch.interval_secs)
    .bind(patch.enabled)
    .bind(patch.datasource_id.is_some()) // $13: touch the datasource?
    .bind(patch.datasource_id.flatten()) // $14: the new value (may be NULL)
    .bind(patch.channel_ids.as_deref()) // $15
    .bind(&patch.message_template) // $16
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_invalid)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Delete a detection (its findings cascade). Returns whether a row matched.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_detections WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> DetectionRecord {
    DetectionRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        insight_id: row.get::<Uuid, _>("insight_id"),
        datasource_id: row.get::<Option<Uuid>, _>("datasource_id"),
        sql: row.get::<String, _>("sql"),
        params: row.get::<Value, _>("params"),
        sources: row.get::<Value, _>("sources"),
        flag_column: row.get::<String, _>("flag_column"),
        target_columns: row.get::<Vec<String>, _>("target_columns"),
        value_column: row.get::<Option<String>, _>("value_column"),
        for_secs: row.get::<i32, _>("for_secs"),
        interval_secs: row.get::<i32, _>("interval_secs"),
        enabled: row.get::<bool, _>("enabled"),
        channel_ids: row.get::<Vec<Uuid>, _>("channel_ids"),
        message_template: row.get::<Option<String>, _>("message_template"),
    }
}

/// A unique-name collision is a `Conflict`; a failed `insight_id` foreign key
/// (the insight does not exist in the tenant) is a caller `Invalid`, not an
/// internal error.
fn conflict_or_invalid(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a detection with that name already exists".into(),
            };
        }
        if db.is_foreign_key_violation() {
            return Error::Invalid {
                message: "detection references an insight that does not exist".into(),
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
