//! Tenant-scoped tagging for any entity.
//!
//! A tag is a `key` + optional `value` attached to an `(entity_type,
//! entity_id)` pair. One table backs every taggable noun; the target is
//! referenced, not foreign-keyed, so entities owned by other layers (users,
//! teams) can be tagged too. Every function runs inside a tenant-bound
//! transaction so RLS isolates the rows — see [`crate::tenant_tx`].
//!
//! There is no DB cascade (a polymorphic, partly-external reference can't have
//! one), so when an entity is deleted its tags are swept with
//! [`delete_for_entity`], called from that entity's delete path.

use sqlx::{PgPool, Row};
use starter_spi::Error;

use crate::tenant_tx;

mod record;

pub use record::{EntityRef, TagRecord, TaggedEntity};

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// Replace the full tag set on an entity with `tags` in one transaction: delete
/// the entity's existing tags, then insert the new set. A full replace (rather
/// than an upsert-per-key) makes the call idempotent and lets the caller drop a
/// tag by omitting it — the editor sends the whole set it wants to persist.
pub async fn set_for_entity(
    pool: &PgPool,
    tenant_id: &str,
    entity: &EntityRef,
    tags: &[TagRecord],
) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query("DELETE FROM nexus_tags WHERE entity_type = $1 AND entity_id = $2")
        .bind(&entity.entity_type)
        .bind(&entity.entity_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    for tag in tags {
        sqlx::query(
            "INSERT INTO nexus_tags (tenant_id, entity_type, entity_id, key, value) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(tenant_id)
        .bind(&entity.entity_type)
        .bind(&entity.entity_id)
        .bind(&tag.key)
        .bind(&tag.value)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    }
    tx.commit().await.map_err(internal)?;
    Ok(())
}

/// The tags on one entity, ordered by key for a stable display.
pub async fn list_for_entity(
    pool: &PgPool,
    tenant_id: &str,
    entity: &EntityRef,
) -> Result<Vec<TagRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT key, value FROM nexus_tags \
         WHERE entity_type = $1 AND entity_id = $2 ORDER BY key",
    )
    .bind(&entity.entity_type)
    .bind(&entity.entity_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows
        .iter()
        .map(|r| TagRecord {
            key: r.get::<String, _>("key"),
            value: r.get::<Option<String>, _>("value"),
        })
        .collect())
}

/// Drop every tag on an entity. Called from the entity's delete path to sweep
/// tags the DB can't cascade. Returns the number removed.
pub async fn delete_for_entity(
    pool: &PgPool,
    tenant_id: &str,
    entity: &EntityRef,
) -> Result<u64, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_tags WHERE entity_type = $1 AND entity_id = $2")
        .bind(&entity.entity_type)
        .bind(&entity.entity_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected())
}

/// Reverse lookup: the entities of `entity_type` carrying a tag with `key`, and
/// — when `value` is `Some` — that exact value. A `None` value matches any
/// value for the key (i.e. "tagged with `key` at all"). Powers tag-filtered
/// listings and filter chips.
pub async fn entities_with_tag(
    pool: &PgPool,
    tenant_id: &str,
    entity_type: &str,
    key: &str,
    value: Option<&str>,
) -> Result<Vec<TaggedEntity>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // `value IS NULL OR value = $3` lets a `None` filter match every value for
    // the key while a `Some` pins it exactly; the bind is the filter, not the
    // row's stored value.
    let rows = sqlx::query(
        "SELECT entity_type, entity_id FROM nexus_tags \
         WHERE entity_type = $1 AND key = $2 AND ($3::text IS NULL OR value = $3) \
         ORDER BY entity_id",
    )
    .bind(entity_type)
    .bind(key)
    .bind(value)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows
        .iter()
        .map(|r| TaggedEntity {
            entity_type: r.get::<String, _>("entity_type"),
            entity_id: r.get::<String, _>("entity_id"),
        })
        .collect())
}

/// The distinct tag keys in use across the tenant, for autocomplete. Ordered.
pub async fn distinct_keys(pool: &PgPool, tenant_id: &str) -> Result<Vec<String>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query("SELECT DISTINCT key FROM nexus_tags ORDER BY key")
        .fetch_all(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(|r| r.get::<String, _>("key")).collect())
}
