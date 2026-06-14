//! Update and reorder/reparent nav nodes.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::fetch::row_to_record;
use super::record::{NavNodePatch, NavNodeRecord};
use crate::tenant_tx;

/// Apply a partial update to a nav node and return the updated row. `None`
/// fields are left unchanged. `parent_id` is three-valued (leave / move-under /
/// re-root) and so cannot ride COALESCE — it is bound with a guard flag.
/// `context`/`icon`/`accent` set to `Some(None)` clear the column (e.g.
/// retargeting a dashboard node to a group drops its context). A self-parent or
/// a cross-tenant parent is rejected (`Invalid`). An absent (or another
/// tenant's) id yields `Ok(None)`.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &NavNodePatch,
) -> Result<Option<NavNodeRecord>, Error> {
    // Reparenting a node under itself would create a one-node cycle; reject it
    // before touching the DB. Deeper cycles are prevented by the tree only ever
    // being built top-down from roots, so a longer cycle is unreachable here.
    if let Some(Some(parent)) = patch.parent_id {
        if parent == id {
            return Err(Error::Invalid {
                message: "a nav node cannot be its own parent".into(),
            });
        }
    }
    let (set_parent, parent) = match patch.parent_id {
        Some(p) => (true, p),
        None => (false, None),
    };
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "UPDATE nexus_nav_nodes SET \
           title      = COALESCE($2, title), \
           sort_order = COALESCE($3, sort_order), \
           target     = COALESCE($4, target), \
           parent_id  = CASE WHEN $5  THEN $6  ELSE parent_id END, \
           context    = CASE WHEN $7  THEN $8  ELSE context   END, \
           icon       = CASE WHEN $9  THEN $10 ELSE icon      END, \
           accent     = CASE WHEN $11 THEN $12 ELSE accent    END \
         WHERE id = $1 \
         RETURNING id, tenant_id, parent_id, title, sort_order, target, context, icon, accent",
    )
    .bind(id)
    .bind(patch.title.as_ref())
    .bind(patch.sort_order)
    .bind(patch.target.as_ref())
    .bind(set_parent)
    .bind(parent)
    .bind(patch.context.is_some())
    .bind(patch.context.as_ref().and_then(|o| o.as_ref()))
    .bind(patch.icon.is_some())
    .bind(patch.icon.as_ref().and_then(|o| o.as_ref()))
    .bind(patch.accent.is_some())
    .bind(patch.accent.as_ref().and_then(|o| o.as_ref()))
    .fetch_optional(&mut *tx)
    .await
    .map_err(bad_parent_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.as_ref().map(row_to_record))
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
