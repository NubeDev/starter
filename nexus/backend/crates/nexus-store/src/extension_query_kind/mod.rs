//! Extension-contributed query-kind persistence — the *third source* the kinds
//! dispatcher resolves, beside the built-in file pack and the tenant-authored
//! overlay (`query_kind`).
//!
//! Unlike [`crate::query_kind`], these rows are **global, not tenant-scoped**:
//! an extension is installed once for the whole deployment (admin-gated), so its
//! kinds apply to every tenant exactly like the file pack does. There is no
//! `tenant_id` column and no RLS — so, unlike the tenant store, these functions
//! run on a bare pooled connection rather than a [`crate::tenant_tx`]. The
//! dispatcher still binds `$caller_tenant_id` at run time, so a kind reading a
//! tenant-scoped table is filtered to the *caller's* tenant.
//!
//! Schema: `migrations/nexus/1801_extension_query_kinds.sql`. The
//! install/contribution path lint-validated the SQL before it reached here
//! (declared `$param`s, `$caller_tenant_id`-guarded tables); the store only
//! persists, it does not re-validate.
//!
//! Lifecycle: an extension's `contributes.warehouse_templates[]` are
//! [`upsert`]ed on install/contribution; the WS-14 cleanup provider calls
//! [`delete_by_extension`] on uninstall+purge, and [`count_by_extension`] /
//! [`list_by_extension`] back the dry-run cleanup manifest.

use serde_json::Value;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

/// An extension-contributed query-kind as stored. Mirrors
/// [`crate::query_kind::QueryKindRecord`] but carries `extension_id` (the owner)
/// in place of `tenant_id` (extension kinds are global).
#[derive(Debug, Clone)]
pub struct ExtensionQueryKindRecord {
    pub id: Uuid,
    pub extension_id: String,
    pub name: String,
    pub sql: String,
    pub params_schema: Value,
    pub datasource_kind: String,
    pub tables: Vec<String>,
    pub datasource_binding: Option<String>,
    pub description: Option<String>,
}

/// A query-kind an extension contributes. `name` is globally unique across all
/// extensions (the install path rejects a clash before calling [`upsert`]).
#[derive(Debug, Clone)]
pub struct NewExtensionQueryKind {
    pub name: String,
    pub sql: String,
    pub params_schema: Value,
    pub datasource_kind: String,
    pub tables: Vec<String>,
    pub datasource_binding: Option<String>,
    pub description: Option<String>,
}

/// Insert or replace the contributed kind named `new.name`, recording
/// `extension_id` as its owner. Upsert (not insert) so a re-install of the same
/// extension is idempotent: the same `name` re-lands the latest definition
/// rather than failing on the global `UNIQUE (name)`.
///
/// A name already owned by a *different* extension is a [`Error::Conflict`] —
/// two extensions cannot contribute the same global kind name.
pub async fn upsert(
    pool: &PgPool,
    extension_id: &str,
    new: &NewExtensionQueryKind,
) -> Result<ExtensionQueryKindRecord, Error> {
    let row = sqlx::query(
        "INSERT INTO nexus_extension_query_kinds \
            (extension_id, name, sql, params_schema, datasource_kind, tables, \
             datasource_binding, description) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
         ON CONFLICT (name) DO UPDATE SET \
            sql                = EXCLUDED.sql, \
            params_schema      = EXCLUDED.params_schema, \
            datasource_kind    = EXCLUDED.datasource_kind, \
            tables             = EXCLUDED.tables, \
            datasource_binding = EXCLUDED.datasource_binding, \
            description        = EXCLUDED.description \
         WHERE nexus_extension_query_kinds.extension_id = EXCLUDED.extension_id \
         RETURNING id",
    )
    .bind(extension_id)
    .bind(&new.name)
    .bind(&new.sql)
    .bind(&new.params_schema)
    .bind(&new.datasource_kind)
    .bind(&new.tables)
    .bind(&new.datasource_binding)
    .bind(&new.description)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;

    // `fetch_optional` is `None` when the `ON CONFLICT … WHERE` guard fails —
    // i.e. a row with this `name` exists but is owned by another extension, so
    // no row was updated and none returned. That is the cross-owner clash.
    let id = row
        .ok_or_else(|| Error::Conflict {
            message: format!(
                "query-kind `{}` is already contributed by another extension",
                new.name
            ),
        })?
        .get::<Uuid, _>("id");

    Ok(ExtensionQueryKindRecord {
        id,
        extension_id: extension_id.to_string(),
        name: new.name.clone(),
        sql: new.sql.clone(),
        params_schema: new.params_schema.clone(),
        datasource_kind: new.datasource_kind.clone(),
        tables: new.tables.clone(),
        datasource_binding: new.datasource_binding.clone(),
        description: new.description.clone(),
    })
}

/// Fetch one extension-contributed kind by name. `Ok(None)` when no row matches
/// — the dispatcher calls this on a file-pack miss and treats a missing kind as
/// "fall through" to the tenant overlay, not an error.
pub async fn get_by_name(
    pool: &PgPool,
    name: &str,
) -> Result<Option<ExtensionQueryKindRecord>, Error> {
    let row = sqlx::query(
        "SELECT id, extension_id, name, sql, params_schema, datasource_kind, tables, \
                datasource_binding, description \
         FROM nexus_extension_query_kinds WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(internal)?;
    Ok(row.as_ref().map(row_to_kind))
}

/// List every kind owned by `extension_id`, name-ordered. Backs the dry-run
/// cleanup manifest the admin UI shows before a purge.
pub async fn list_by_extension(
    pool: &PgPool,
    extension_id: &str,
) -> Result<Vec<ExtensionQueryKindRecord>, Error> {
    let rows = sqlx::query(
        "SELECT id, extension_id, name, sql, params_schema, datasource_kind, tables, \
                datasource_binding, description \
         FROM nexus_extension_query_kinds WHERE extension_id = $1 ORDER BY name",
    )
    .bind(extension_id)
    .fetch_all(pool)
    .await
    .map_err(internal)?;
    Ok(rows.iter().map(row_to_kind).collect())
}

/// Count the kinds owned by `extension_id`. Cheaper than [`list_by_extension`]
/// when the cleanup manifest only needs a tally.
pub async fn count_by_extension(pool: &PgPool, extension_id: &str) -> Result<i64, Error> {
    let row = sqlx::query(
        "SELECT count(*) AS n FROM nexus_extension_query_kinds WHERE extension_id = $1",
    )
    .bind(extension_id)
    .fetch_one(pool)
    .await
    .map_err(internal)?;
    Ok(row.get::<i64, _>("n"))
}

/// Delete every kind owned by `extension_id`, returning how many rows went.
/// Idempotent — a second call on an already-clean extension deletes nothing and
/// returns `0`, satisfying the WS-14 cleanup re-purge contract.
pub async fn delete_by_extension(pool: &PgPool, extension_id: &str) -> Result<u64, Error> {
    let result = sqlx::query("DELETE FROM nexus_extension_query_kinds WHERE extension_id = $1")
        .bind(extension_id)
        .execute(pool)
        .await
        .map_err(internal)?;
    Ok(result.rows_affected())
}

fn row_to_kind(row: &sqlx::postgres::PgRow) -> ExtensionQueryKindRecord {
    ExtensionQueryKindRecord {
        id: row.get::<Uuid, _>("id"),
        extension_id: row.get::<String, _>("extension_id"),
        name: row.get::<String, _>("name"),
        sql: row.get::<String, _>("sql"),
        params_schema: row.get::<serde_json::Value, _>("params_schema"),
        datasource_kind: row.get::<String, _>("datasource_kind"),
        tables: row.get::<Vec<String>, _>("tables"),
        datasource_binding: row.get::<Option<String>, _>("datasource_binding"),
        description: row.get::<Option<String>, _>("description"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
