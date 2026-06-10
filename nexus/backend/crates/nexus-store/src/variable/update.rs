//! Partial update of a dashboard variable within a tenant.

use sqlx::types::Json;
use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::fetch::row_to_record;
use super::record::{VariablePatch, VariableRecord};
use crate::tenant_tx;

/// Update variable `id` within `tenant_id`. Each `None` field is left unchanged
/// via COALESCE, so one statement covers any subset — the common case being a
/// `current`-only patch when the user picks a new value in the bar. Returns the
/// updated record, or `None` when RLS hides it / it does not exist. Renaming to a
/// name already used on the same dashboard is a `Conflict`.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &VariablePatch,
) -> Result<Option<VariableRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "UPDATE nexus_dashboard_variables SET \
           name           = COALESCE($2, name), \
           label          = COALESCE($3, label), \
           kind           = COALESCE($4, kind), \
           options_config = COALESCE($5, options_config), \
           current        = COALESCE($6, current), \
           multi          = COALESCE($7, multi), \
           include_all    = COALESCE($8, include_all), \
           hidden         = COALESCE($9, hidden), \
           sort_order     = COALESCE($10, sort_order) \
         WHERE id = $1 \
         RETURNING id, dashboard_id, name, label, kind, options_config, current, \
                   multi, include_all, hidden, sort_order",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.label)
    .bind(&patch.kind)
    .bind(&patch.options_config)
    .bind(patch.current.as_ref().map(Json))
    .bind(patch.multi)
    .bind(patch.include_all)
    .bind(patch.hidden)
    .bind(patch.sort_order)
    .fetch_optional(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.as_ref().map(row_to_record))
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a variable with that name already exists on this dashboard".into(),
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
