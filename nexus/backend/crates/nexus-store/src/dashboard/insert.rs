//! Create a dashboard for a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DashboardRecord, NewDashboard};
use crate::tenant_tx;

/// Insert a dashboard. A slug already used in the tenant is a `Conflict`, not a
/// silent alias.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewDashboard,
) -> Result<DashboardRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_dashboards (tenant_id, slug, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.slug)
    .bind(&new.name)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(DashboardRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        slug: new.slug.clone(),
        name: new.name.clone(),
    })
}

/// A unique-violation on (tenant_id, slug) is the caller's conflict; anything
/// else is ours.
fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a dashboard with that slug already exists".into(),
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
