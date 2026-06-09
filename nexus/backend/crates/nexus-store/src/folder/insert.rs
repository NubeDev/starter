//! Create a folder for a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{FolderRecord, NewFolder};
use crate::tenant_tx;

/// Insert a folder. A `parent_id` that names a folder in another tenant (or no
/// folder at all) is a caller error — RLS hides the row so the FK check fails
/// and we surface it as `Invalid` rather than leaking existence.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewFolder,
) -> Result<FolderRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_folders (tenant_id, parent_id, name) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(tenant_id)
    .bind(new.parent_id)
    .bind(&new.name)
    .fetch_one(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(FolderRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        parent_id: new.parent_id,
        name: new.name.clone(),
    })
}

/// Insert a folder under a caller-supplied id rather than a fresh one. Used by
/// the undo path (resurrect-on-undo-of-delete / redo-of-create), which must
/// restore the **original** id so any rows that referenced it can be re-filed.
/// Same parent-FK semantics as [`insert`].
pub async fn insert_with_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    new: &NewFolder,
) -> Result<FolderRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_folders (id, tenant_id, parent_id, name) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(new.parent_id)
    .bind(&new.name)
    .execute(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(FolderRecord {
        id,
        tenant_id: tenant_id.to_string(),
        parent_id: new.parent_id,
        name: new.name.clone(),
    })
}

/// A foreign-key violation on `parent_id` means the parent is absent or another
/// tenant's; anything else is ours.
fn bad_parent_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_foreign_key_violation() {
            return Error::Invalid {
                message: "no such parent folder".into(),
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
