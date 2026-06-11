//! WS-17 Wave B — extension access to configured datasources.
//!
//! Two host methods let an extension run full CRUD against a nexus datasource it
//! is authorised for, tenant-scoped exactly like the human
//! `POST /datasources/{id}/query` route:
//!
//! - `datasource.query` — a read. Runs in a `READ ONLY` transaction with the
//!   server `statement_timeout` guard (writes/DDL are rejected by Postgres
//!   itself, not by inspecting the text), so an extension cannot mutate through
//!   the read method.
//! - `datasource.execute` — a write/DDL. Bounded by the **ownership-prefix rule**
//!   (a `CREATE TABLE` must target `<sanitized_ext_id>__<table>`) and, for CRUD
//!   against a non-owned table, the operator `allow_foreign_tables` grant
//!   (WS-17 §4.2 / Q3). Owned-prefix tables are writable freely.
//!
//! The datasource must be in the calling extension's `datasource` grant
//! (`Capability::Datasource { datasources, .. }`) AND resolve within the
//! caller's tenant — the same `datasource::get(tenant, id)` lookup the human
//! route uses, so an extension can never reach another tenant's datasource.

use std::collections::BTreeSet;

use serde_json::Value as JsonValue;
use sqlx::{postgres::PgArguments, Arguments, Executor, PgPool, Postgres};
use starter_ext_spi::datasource::{
    DatasourceExecuteRequest, DatasourceExecuteResponse, DatasourceQueryRequest,
    DatasourceQueryResponse,
};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::warehouse::Row;
use starter_ext_spi::{Capability, Error as ExtError, ExtensionId, Manifest, Result as ExtResult};
use uuid::Uuid;

use super::warehouse::sanitize_extension_id;
use crate::state::AppState;

/// The calling extension's datasource grant: the allowed datasource ids and the
/// foreign-table flag. `None` when the extension declares no `datasource`
/// capability at all ⇒ every call refused.
struct DatasourceGrant {
    datasources: BTreeSet<String>,
    allow_foreign_tables: bool,
}

fn grant_of(manifest: &Manifest) -> Option<DatasourceGrant> {
    manifest.capabilities.iter().find_map(|c| match c {
        Capability::Datasource {
            datasources,
            allow_foreign_tables,
        } => Some(DatasourceGrant {
            datasources: datasources.iter().cloned().collect(),
            allow_foreign_tables: *allow_foreign_tables,
        }),
        _ => None,
    })
}

/// Resolve the caller's tenant, refusing a tenant-less caller (hard deny, like
/// every tenant-scoped host method).
fn caller_tenant(caller: Option<&CallerIdentity>) -> ExtResult<String> {
    caller
        .and_then(|c| c.tenant_id.clone())
        .ok_or_else(|| ExtError::extension_internal("datasource.* requires a tenant-scoped caller"))
}

/// Shared pre-flight: pull the grant, gate the datasource id, resolve the record
/// within the caller's tenant, and build (or reuse) the pool. Returns the pool
/// and the grant for the per-method body.
async fn resolve(
    state: &AppState,
    extension: &ExtensionId,
    tenant: &str,
    actor: &str,
    datasource_id: &str,
) -> ExtResult<(PgPool, DatasourceGrant)> {
    let grant = state
        .extensions
        .get_by_id_str(extension.as_str())
        .and_then(|r| r.manifest.as_ref())
        .and_then(grant_of)
        .ok_or_else(|| {
            ExtError::capability(format!(
                "datasource: extension {:?} declares no `datasource` capability",
                extension.as_str()
            ))
        })?;
    if !grant.datasources.contains(datasource_id) {
        return Err(ExtError::capability(format!(
            "datasource: id {datasource_id:?} is not in extension {:?}'s grant",
            extension.as_str()
        )));
    }
    let id = Uuid::parse_str(datasource_id).map_err(|e| {
        ExtError::extension_internal(format!("datasource id {datasource_id:?} is not a UUID: {e}"))
    })?;
    let rec = nexus_store::datasource::get(&state.metadata, tenant, id)
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource lookup: {e}")))?
        .ok_or_else(|| {
            // Not found *in this tenant* — never reveal another tenant's id.
            ExtError::extension_internal(format!(
                "datasource {datasource_id} not found for the caller's tenant"
            ))
        })?;
    let pool = state
        .datasource_pools
        .get_or_connect(&state.metadata, &state.envelope, tenant, actor, &rec)
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource connect: {e}")))?;
    Ok((pool, grant))
}

/// `datasource.query` → run a read against a named datasource, returning rows.
pub async fn query(
    state: &AppState,
    extension: &ExtensionId,
    params: JsonValue,
    caller: Option<&CallerIdentity>,
) -> ExtResult<JsonValue> {
    let req: DatasourceQueryRequest = serde_json::from_value(params)
        .map_err(|e| ExtError::extension_internal(format!("datasource.query params: {e}")))?;
    let tenant = caller_tenant(caller)?;
    let actor = caller
        .and_then(|c| c.user_id.clone())
        .unwrap_or_else(|| "system".to_string());
    let (pool, _grant) = resolve(state, extension, &tenant, &actor, &req.datasource_id).await?;

    let rows = run_read(&pool, &req.sql, &req.params, state.guards).await?;
    serde_json::to_value(DatasourceQueryResponse { rows })
        .map_err(|e| ExtError::extension_internal(format!("datasource.query response: {e}")))
}

/// `datasource.execute` → run a write/DDL against a named datasource. Enforces
/// the ownership-prefix rule for CREATE and the foreign-table grant for CRUD
/// against non-owned tables.
pub async fn execute(
    state: &AppState,
    extension: &ExtensionId,
    params: JsonValue,
    caller: Option<&CallerIdentity>,
) -> ExtResult<JsonValue> {
    let req: DatasourceExecuteRequest = serde_json::from_value(params)
        .map_err(|e| ExtError::extension_internal(format!("datasource.execute params: {e}")))?;
    let tenant = caller_tenant(caller)?;
    let actor = caller
        .and_then(|c| c.user_id.clone())
        .unwrap_or_else(|| "system".to_string());
    let (pool, grant) = resolve(state, extension, &tenant, &actor, &req.datasource_id).await?;

    enforce_ownership(extension, &req.statement, grant.allow_foreign_tables)?;

    let rows_affected = run_write(&pool, &req.statement, &req.params, state.guards).await?;
    serde_json::to_value(DatasourceExecuteResponse { rows_affected })
        .map_err(|e| ExtError::extension_internal(format!("datasource.execute response: {e}")))
}

// ---------------------------------------------------------------------------
// Ownership-prefix rule (WS-17 §4.2 / Q3).
// ---------------------------------------------------------------------------

/// Refuse a write that would CREATE a table without the `<ext>__` prefix, or
/// (unless `allow_foreign_tables`) CRUD a table that lacks the prefix.
///
/// Best-effort parse of the statement's target table name; the prefix is the
/// ownership marker, so a CREATE must always carry it. For DML
/// (INSERT/UPDATE/DELETE) against an existing **non-owned** table, the broader
/// `allow_foreign_tables` operator grant is required — owned-prefix tables are
/// always allowed.
fn enforce_ownership(
    extension: &ExtensionId,
    statement: &str,
    allow_foreign_tables: bool,
) -> ExtResult<()> {
    let prefix = format!("{}__", sanitize_extension_id(extension));
    let head = first_keyword(statement);
    let target = match head.as_str() {
        "create" => target_after(statement, &["table"]),
        "insert" => target_after(statement, &["into"]),
        "update" => target_after(statement, &["update"]),
        "delete" => target_after(statement, &["from"]),
        // ALTER/DROP/TRUNCATE and anything else: treat like DML — owned freely,
        // foreign only under the grant.
        _ => target_after_any(statement),
    };

    match target {
        Some(table) => {
            let bare = strip_schema_and_quotes(&table);
            let owned = bare.starts_with(&prefix);
            if head == "create" && !owned {
                return Err(ExtError::capability(format!(
                    "datasource.execute: CREATE must target an `{prefix}` table \
                     (ownership prefix); got {bare:?}"
                )));
            }
            if !owned && !allow_foreign_tables {
                return Err(ExtError::capability(format!(
                    "datasource.execute: table {bare:?} is not owned by this extension \
                     (no `{prefix}` prefix) and `allow_foreign_tables` is not granted"
                )));
            }
            Ok(())
        }
        None => {
            // Could not identify a target. If foreign tables are allowed the
            // statement runs; otherwise refuse rather than run an unbounded
            // mutation we cannot attribute to an owned table.
            if allow_foreign_tables {
                Ok(())
            } else {
                Err(ExtError::capability(
                    "datasource.execute: could not identify the target table to \
                     enforce the ownership prefix; grant `allow_foreign_tables` to \
                     run arbitrary statements",
                ))
            }
        }
    }
}

/// Lowercased first SQL keyword.
fn first_keyword(sql: &str) -> String {
    sql.split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// The token following the first occurrence of any keyword in `keywords`
/// (case-insensitive). Skips an optional `IF NOT EXISTS`.
fn target_after(sql: &str, keywords: &[&str]) -> Option<String> {
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    let lower: Vec<String> = tokens.iter().map(|t| t.to_ascii_lowercase()).collect();
    for (i, t) in lower.iter().enumerate() {
        if keywords.contains(&t.as_str()) {
            let mut j = i + 1;
            // Skip CREATE TABLE IF NOT EXISTS …
            while j < lower.len() && matches!(lower[j].as_str(), "if" | "not" | "exists") {
                j += 1;
            }
            if j < tokens.len() {
                return Some(tokens[j].to_string());
            }
        }
    }
    None
}

/// Fallback: the second token (most DML names the table as `<verb> <table>` or
/// `<verb> <kw> <table>`). Returns `None` if there is no second token.
fn target_after_any(sql: &str) -> Option<String> {
    sql.split_whitespace().nth(1).map(|s| s.to_string())
}

/// Strip a leading `schema.` and surrounding quotes/backticks, and a trailing
/// `(` (e.g. `create table foo(` → `foo`), returning the bare table identifier
/// lowercased for the prefix check (identifiers are case-insensitive unquoted).
fn strip_schema_and_quotes(raw: &str) -> String {
    let raw = raw.trim_end_matches('(').trim();
    let raw = raw.rsplit('.').next().unwrap_or(raw);
    raw.trim_matches(|c| c == '"' || c == '`' || c == '\'')
        .to_ascii_lowercase()
}

// ---------------------------------------------------------------------------
// Execution — positional-param read / write against the datasource pool.
// ---------------------------------------------------------------------------

/// Run a read in a `READ ONLY` transaction with the statement-timeout guard, and
/// collect rows (bounded by `max_rows`). The READ ONLY transaction is the
/// security boundary — any write phrased as a "query" is rejected by Postgres.
async fn run_read(
    pool: &PgPool,
    sql: &str,
    params: &[JsonValue],
    guards: nexus_store::QueryGuards,
) -> ExtResult<Vec<Row>> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource txn: {e}")))?;
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(|e| ExtError::extension_internal(format!("read-only: {e}")))?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    tx.execute(format!("SET LOCAL statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(|e| ExtError::extension_internal(format!("timeout guard: {e}")))?;

    let rows = sqlx::query_with::<Postgres, _>(sql, bind_params(params)?)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource.query: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ExtError::extension_internal(format!("commit: {e}")))?;

    let max = guards.max_rows as usize;
    let out = rows
        .iter()
        .take(max)
        .filter_map(|r| match nexus_store::row_to_object(r) {
            JsonValue::Object(map) => Some(Row::from_map(map)),
            _ => None,
        })
        .collect();
    Ok(out)
}

/// Run a write/DDL statement (autocommit) with the statement-timeout guard,
/// returning the affected-row count.
async fn run_write(
    pool: &PgPool,
    statement: &str,
    params: &[JsonValue],
    guards: nexus_store::QueryGuards,
) -> ExtResult<u64> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource conn: {e}")))?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    conn.execute(format!("SET statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(|e| ExtError::extension_internal(format!("timeout guard: {e}")))?;
    let res = sqlx::query_with::<Postgres, _>(statement, bind_params(params)?)
        .execute(&mut *conn)
        .await
        .map_err(|e| ExtError::extension_internal(format!("datasource.execute: {e}")))?;
    Ok(res.rows_affected())
}

/// Bind positional JSON params as `$1..$N`. Dispatches on the JSON shape (text /
/// int / float / bool / null); arrays and objects bind as JSON(B).
fn bind_params(params: &[JsonValue]) -> ExtResult<PgArguments> {
    let mut args = PgArguments::default();
    for (i, v) in params.iter().enumerate() {
        let map_bind = |e: sqlx::error::BoxDynError| {
            ExtError::extension_internal(format!("bind ${}: {e}", i + 1))
        };
        if v.is_null() {
            args.add::<Option<String>>(None).map_err(map_bind)?;
        } else if let Some(s) = v.as_str() {
            args.add(s.to_owned()).map_err(map_bind)?;
        } else if let Some(n) = v.as_i64() {
            args.add(n).map_err(map_bind)?;
        } else if let Some(f) = v.as_f64() {
            args.add(f).map_err(map_bind)?;
        } else if let Some(b) = v.as_bool() {
            args.add(b).map_err(map_bind)?;
        } else {
            args.add(v.clone()).map_err(map_bind)?;
        }
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext() -> ExtensionId {
        ExtensionId::new("com.acme.devices").unwrap()
    }

    #[test]
    fn create_must_be_prefixed() {
        // A CREATE of a non-owned table is refused even with the foreign grant
        // off; the owned-prefix CREATE passes.
        assert!(enforce_ownership(&ext(), "CREATE TABLE widgets (id int)", false).is_err());
        assert!(enforce_ownership(
            &ext(),
            "CREATE TABLE com_acme_devices__widgets (id int)",
            false
        )
        .is_ok());
        // IF NOT EXISTS is skipped when finding the target.
        assert!(enforce_ownership(
            &ext(),
            "create table if not exists com_acme_devices__t (id int)",
            false
        )
        .is_ok());
    }

    #[test]
    fn dml_against_foreign_table_needs_grant() {
        // Without the grant, an INSERT into a non-owned table is refused…
        assert!(enforce_ownership(&ext(), "INSERT INTO sales VALUES (1)", false).is_err());
        // …but allowed with it.
        assert!(enforce_ownership(&ext(), "INSERT INTO sales VALUES (1)", true).is_ok());
        // An owned-prefix DML is always allowed.
        assert!(
            enforce_ownership(&ext(), "INSERT INTO com_acme_devices__log VALUES (1)", false).is_ok()
        );
    }

    #[test]
    fn schema_and_quotes_stripped() {
        assert_eq!(strip_schema_and_quotes("public.\"Foo\""), "foo");
        assert_eq!(strip_schema_and_quotes("com_acme_devices__t("), "com_acme_devices__t");
    }

    #[test]
    fn delete_and_update_targets_resolved() {
        assert!(enforce_ownership(&ext(), "DELETE FROM widgets WHERE id = 1", false).is_err());
        assert!(
            enforce_ownership(&ext(), "UPDATE com_acme_devices__t SET x = 1", false).is_ok()
        );
    }
}
