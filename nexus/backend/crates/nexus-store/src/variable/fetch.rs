//! Read dashboard variables — list a dashboard's variables, and get one by id.

use sqlx::types::Json;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::VariableRecord;
use crate::tenant_tx;

/// List a dashboard's variables in bar/resolution order (`sort_order`, then
/// creation order). Tenant-scoped via RLS; another tenant's rows are invisible.
pub async fn list_for_dashboard(
    pool: &PgPool,
    tenant_id: &str,
    dashboard_id: Uuid,
) -> Result<Vec<VariableRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, dashboard_id, name, label, kind, options_config, current, \
         multi, include_all, hidden, sort_order \
         FROM nexus_dashboard_variables WHERE dashboard_id = $1 \
         ORDER BY sort_order ASC, created_at ASC",
    )
    .bind(dashboard_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one variable by id within the tenant. `Ok(None)` covers both absent and
/// another tenant's — existence is not leaked.
pub async fn by_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<VariableRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, dashboard_id, name, label, kind, options_config, current, \
         multi, include_all, hidden, sort_order \
         FROM nexus_dashboard_variables WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

/// Map a row to a record, lifting the `current` jsonb array into a `Vec<String>`.
pub(super) fn row_to_record(row: &sqlx::postgres::PgRow) -> VariableRecord {
    VariableRecord {
        id: row.get::<Uuid, _>("id"),
        dashboard_id: row.get::<Uuid, _>("dashboard_id"),
        name: row.get::<String, _>("name"),
        label: row.get::<Option<String>, _>("label"),
        kind: row.get::<String, _>("kind"),
        options_config: row.get::<serde_json::Value, _>("options_config"),
        current: row.get::<Json<Vec<String>>, _>("current").0,
        multi: row.get::<bool, _>("multi"),
        include_all: row.get::<bool, _>("include_all"),
        hidden: row.get::<bool, _>("hidden"),
        sort_order: row.get::<i32, _>("sort_order"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
