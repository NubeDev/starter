//! Create a variable on a dashboard.

use sqlx::types::Json;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewVariable, VariableRecord};
use crate::tenant_tx;

/// Insert a variable. A name already used on the dashboard is a `Conflict`, not
/// a silent overwrite. The `tenant_id` is bound from the request principal so RLS
/// applies; the `dashboard_id` FK ties the row to a dashboard the tenant owns.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewVariable,
) -> Result<VariableRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_dashboard_variables \
         (tenant_id, dashboard_id, name, label, kind, options_config, current, \
          multi, include_all, hidden, sort_order) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id",
    )
    .bind(tenant_id)
    .bind(new.dashboard_id)
    .bind(&new.name)
    .bind(&new.label)
    .bind(&new.kind)
    .bind(&new.options_config)
    .bind(Json(&new.current))
    .bind(new.multi)
    .bind(new.include_all)
    .bind(new.hidden)
    .bind(new.sort_order)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(VariableRecord {
        id: row.get::<Uuid, _>("id"),
        dashboard_id: new.dashboard_id,
        name: new.name.clone(),
        label: new.label.clone(),
        kind: new.kind.clone(),
        options_config: new.options_config.clone(),
        current: new.current.clone(),
        multi: new.multi,
        include_all: new.include_all,
        hidden: new.hidden,
        sort_order: new.sort_order,
    })
}

/// A unique-violation on (dashboard_id, name) is the caller's conflict; a
/// foreign-key violation means the dashboard is absent/another tenant's, also a
/// caller error; anything else is ours.
fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a variable with that name already exists on this dashboard".into(),
            };
        }
        if db.is_foreign_key_violation() {
            return Error::Invalid {
                message: "no such dashboard".into(),
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
