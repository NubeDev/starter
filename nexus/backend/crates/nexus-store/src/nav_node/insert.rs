//! Create a nav node for a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NavNodeRecord, NewNavNode};
use crate::tenant_tx;

/// Insert a nav node. A `parent_id` that names a node in another tenant (or no
/// node at all) fails the FK check under RLS and surfaces as `Invalid` rather
/// than leaking existence — mirror of the folder rule.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewNavNode,
) -> Result<NavNodeRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_nav_nodes \
            (tenant_id, parent_id, title, sort_order, target, context, icon, accent) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(tenant_id)
    .bind(new.parent_id)
    .bind(&new.title)
    .bind(new.sort_order)
    .bind(&new.target)
    .bind(&new.context)
    .bind(&new.icon)
    .bind(&new.accent)
    .fetch_one(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(NavNodeRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        parent_id: new.parent_id,
        title: new.title.clone(),
        sort_order: new.sort_order,
        target: new.target.clone(),
        context: new.context.clone(),
        icon: new.icon.clone(),
        accent: new.accent.clone(),
    })
}

/// Insert a nav node under a caller-supplied id rather than a fresh one. Used by
/// the undo path (resurrect-on-undo-of-delete / redo-of-create), which must
/// restore the **original** id so any child rows that referenced it can be
/// re-parented. Same parent-FK semantics as [`insert`].
pub async fn insert_with_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    new: &NewNavNode,
) -> Result<NavNodeRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_nav_nodes \
            (id, tenant_id, parent_id, title, sort_order, target, context, icon, accent) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(new.parent_id)
    .bind(&new.title)
    .bind(new.sort_order)
    .bind(&new.target)
    .bind(&new.context)
    .bind(&new.icon)
    .bind(&new.accent)
    .execute(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(NavNodeRecord {
        id,
        tenant_id: tenant_id.to_string(),
        parent_id: new.parent_id,
        title: new.title.clone(),
        sort_order: new.sort_order,
        target: new.target.clone(),
        context: new.context.clone(),
        icon: new.icon.clone(),
        accent: new.accent.clone(),
    })
}

/// A foreign-key violation on `parent_id` means the parent is absent or another
/// tenant's; anything else is ours.
fn bad_parent_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_foreign_key_violation() {
            return Error::Invalid {
                message: "no such parent nav node".into(),
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
