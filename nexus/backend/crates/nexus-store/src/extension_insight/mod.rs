//! Extension-contributed insight persistence — the global insight registry an
//! installed extension contributes via `contributes.insights[]`, the dual of
//! [`crate::extension_query_kind`] for the post-query insight stage.
//!
//! Like extension query-kinds these rows are **global, not tenant-scoped**: an
//! extension is installed once for the whole deployment (admin-gated), so its
//! insights are available to every tenant exactly like the file-pack query-kinds
//! are. There is no `tenant_id` column and no RLS — so, unlike the tenant
//! [`crate::insight`] store, these functions run on a bare pooled connection
//! rather than a [`crate::tenant_tx`]. The script still runs against the
//! caller's own result rows at query time, so a global definition only ever
//! touches the caller's data.
//!
//! Schema: `migrations/nexus/2201_extension_insights.sql`. The
//! install/contribution path compiled the script against the insight sandbox
//! before it reached here; the store only persists, it does not re-validate.
//!
//! Lifecycle: an extension's `contributes.insights[]` are [`upsert`]ed on
//! install/contribution; the WS-14 cleanup provider calls [`delete_by_extension`]
//! on uninstall+purge, and [`list_by_extension`] backs the dry-run cleanup
//! manifest.

use serde_json::Value;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

/// An extension-contributed insight as stored. Mirrors [`crate::insight::InsightRecord`]
/// but carries `extension_id` (the owner) in place of `tenant_id` (extension
/// insights are global).
#[derive(Debug, Clone)]
pub struct ExtensionInsightRecord {
    pub id: Uuid,
    pub extension_id: String,
    pub name: String,
    pub script: String,
    pub params_schema: Option<Value>,
}

/// An insight an extension contributes. `name` is globally unique across all
/// extensions (the contribution path rejects a clash before calling [`upsert`]).
#[derive(Debug, Clone)]
pub struct NewExtensionInsight {
    pub name: String,
    pub script: String,
    pub params_schema: Option<Value>,
}

/// Insert or replace the contributed insight named `new.name`, recording
/// `extension_id` as its owner. Upsert (not insert) so a re-install of the same
/// extension is idempotent: the same `name` re-lands the latest definition
/// rather than failing on the global `UNIQUE (name)`.
///
/// A name already owned by a *different* extension is a [`Error::Conflict`] —
/// two extensions cannot contribute the same global insight name.
pub async fn upsert(
    pool: &PgPool,
    extension_id: &str,
    new: &NewExtensionInsight,
) -> Result<ExtensionInsightRecord, Error> {
    let row = sqlx::query(
        "INSERT INTO nexus_extension_insights (extension_id, name, script, params_schema) \
         VALUES ($1,$2,$3,$4) \
         ON CONFLICT (name) DO UPDATE SET \
            script        = EXCLUDED.script, \
            params_schema = EXCLUDED.params_schema \
         WHERE nexus_extension_insights.extension_id = EXCLUDED.extension_id \
         RETURNING id",
    )
    .bind(extension_id)
    .bind(&new.name)
    .bind(&new.script)
    .bind(&new.params_schema)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;

    // `fetch_optional` is `None` when the `ON CONFLICT … WHERE` guard fails —
    // a row with this `name` exists but is owned by another extension, so no row
    // was updated and none returned. That is the cross-owner clash.
    let id = row
        .ok_or_else(|| Error::Conflict {
            message: format!(
                "insight `{}` is already contributed by another extension",
                new.name
            ),
        })?
        .get::<Uuid, _>("id");

    Ok(ExtensionInsightRecord {
        id,
        extension_id: extension_id.to_string(),
        name: new.name.clone(),
        script: new.script.clone(),
        params_schema: new.params_schema.clone(),
    })
}

/// Fetch one extension-contributed insight by name. `Ok(None)` when no row
/// matches — the query path treats a missing name as a clean caller error, not
/// an internal failure.
pub async fn get_by_name(
    pool: &PgPool,
    name: &str,
) -> Result<Option<ExtensionInsightRecord>, Error> {
    let row = sqlx::query(
        "SELECT id, extension_id, name, script, params_schema \
         FROM nexus_extension_insights WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;
    Ok(row.as_ref().map(row_to_insight))
}

/// List every insight owned by `extension_id`, name-ordered. Backs the dry-run
/// cleanup manifest the admin UI shows before a purge.
pub async fn list_by_extension(
    pool: &PgPool,
    extension_id: &str,
) -> Result<Vec<ExtensionInsightRecord>, Error> {
    let rows = sqlx::query(
        "SELECT id, extension_id, name, script, params_schema \
         FROM nexus_extension_insights WHERE extension_id = $1 ORDER BY name",
    )
    .bind(extension_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;
    Ok(rows.iter().map(row_to_insight).collect())
}

/// Delete every insight owned by `extension_id`, returning how many rows went.
/// Idempotent — a second call on an already-clean extension deletes nothing and
/// returns `0`, satisfying the WS-14 cleanup re-purge contract.
pub async fn delete_by_extension(pool: &PgPool, extension_id: &str) -> Result<u64, Error> {
    let result = sqlx::query("DELETE FROM nexus_extension_insights WHERE extension_id = $1")
        .bind(extension_id)
        .execute(pool)
        .await
        .map_err(internal)?;
    Ok(result.rows_affected())
}

fn row_to_insight(row: &sqlx::postgres::PgRow) -> ExtensionInsightRecord {
    ExtensionInsightRecord {
        id: row.get::<Uuid, _>("id"),
        extension_id: row.get::<String, _>("extension_id"),
        name: row.get::<String, _>("name"),
        script: row.get::<String, _>("script"),
        params_schema: row.get::<Option<Value>, _>("params_schema"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
