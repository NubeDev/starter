//! Rename / reparent a folder within a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{FolderPatch, FolderRecord};
use crate::tenant_tx;

/// Partial update of folder `id` within `tenant_id`. The name uses COALESCE
/// (None = unchanged). The parent is three-valued — leave / move-under /
/// re-root — which COALESCE can't express, so it is bound directly: when the
/// patch carries `Some(parent)` we set `parent_id = parent` (possibly NULL to
/// re-root); when it carries `None` we leave it via a guard flag. A self-parent
/// or a cross-tenant parent is rejected (`Invalid`).
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &FolderPatch,
) -> Result<Option<FolderRecord>, Error> {
    // Reparenting a folder under itself would create a one-node cycle; reject it
    // before touching the DB. Deeper cycles are prevented by the tree only ever
    // being built top-down from roots, so a longer cycle is unreachable here.
    if let Some(Some(parent)) = patch.parent_id {
        if parent == id {
            return Err(Error::Invalid {
                message: "a folder cannot be its own parent".into(),
            });
        }
    }
    let (set_parent, parent) = match patch.parent_id {
        Some(p) => (true, p),
        None => (false, None),
    };
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "UPDATE nexus_folders SET \
           name      = COALESCE($2, name), \
           parent_id = CASE WHEN $3 THEN $4 ELSE parent_id END \
         WHERE id = $1 \
         RETURNING id, tenant_id, parent_id, name",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(set_parent)
    .bind(parent)
    .fetch_optional(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.map(|r| FolderRecord {
        id: r.get::<Uuid, _>("id"),
        tenant_id: r.get::<String, _>("tenant_id"),
        parent_id: r.get::<Option<Uuid>, _>("parent_id"),
        name: r.get::<String, _>("name"),
    }))
}

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
