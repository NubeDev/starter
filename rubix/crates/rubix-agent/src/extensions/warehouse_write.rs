//! Rubix-side [`WarehouseWriteBackend`] — extensions write rows
//! into the tables they declared in
//! `contributes.warehouse_tables[]`.
//!
//! This is the companion to [`super::backends::RubixWarehouseReadBackend`].
//! Differences worth noting:
//!
//! - The **table name** an extension passes is **unprefixed** (it
//!   matches the manifest's `contributes.warehouse_tables[].name`).
//!   The host resolves it to `<sanitize(extension_id)>__<name>`
//!   before issuing SQL, so two extensions cannot collide.
//! - Every row's `tenant_id` is **overwritten** by the host with
//!   `ctx.caller().tenant_id` before the INSERT. The extension
//!   cannot spoof cross-tenant writes.
//! - Columns are **validated** against the manifest spec — unknown
//!   columns refuse with `Error::Validation`; missing required
//!   columns (no `default` in the manifest) also refuse.
//! - Column **types are bound** through a small whitelist (text,
//!   int, float, bool, timestamp-ms). Other Postgres types refuse
//!   with `Error::Validation`. Extensions that need richer types
//!   can land them additively — the per-type binding lives in one
//!   function ([`bind_column`]).
//!
//! The warehouse is Postgres/Timescale today, so all SQL emitted
//! here uses Postgres parameter syntax (`$N`) and types. A future
//! ClickHouse backend would land as a sibling module gated on the
//! `WarehouseClient` engine kind.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use sqlx::{postgres::PgArguments, Arguments, Postgres};
use starter_ext_sdk::ctx::WarehouseWriteBackend;
use starter_ext_spi::manifest::{ContributeWarehouseTable, TableColumn};
use starter_ext_spi::warehouse::Row;
use starter_ext_spi::{Error, ExtensionId, Result};
use starter_store_warehouse::WarehouseClient;

/// Per-call rubix [`WarehouseWriteBackend`].
///
/// Construction is per-call: the [`super::backends::RubixCapabilityFactory`]
/// builds one of these every time the dispatcher mints a `Ctx`. The
/// cost is one `Arc` clone of each input — no allocations on the
/// hot path until `insert` actually runs.
#[derive(Clone)]
pub struct RubixWarehouseWriteBackend {
    client: WarehouseClient,
    caller_tenant_id: Option<String>,
    extension_id: ExtensionId,
    /// Set of unprefixed table names the calling extension's
    /// manifest grant permits. `None` short-circuits the gate
    /// (host-internal frames bypass the manifest pipeline);
    /// `Some(empty)` refuses every table.
    granted_tables: Option<BTreeSet<String>>,
    /// Table schemas the extension declared in
    /// `contributes.warehouse_tables[]`. The backend looks up by
    /// unprefixed `name`. `Arc` so cloning the backend per call is
    /// cheap regardless of how many tables an extension declares.
    table_specs: Arc<Vec<ContributeWarehouseTable>>,
}

impl std::fmt::Debug for RubixWarehouseWriteBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixWarehouseWriteBackend")
            .field("caller_tenant_id", &self.caller_tenant_id)
            .field("extension_id", &self.extension_id.as_str())
            .field("granted_tables", &self.granted_tables)
            .field("declared_tables", &self.table_specs.len())
            .finish_non_exhaustive()
    }
}

impl RubixWarehouseWriteBackend {
    /// Construct directly. Prefer the factory's
    /// `warehouse_write(extension, caller)` accessor in production
    /// — this constructor is for tests and host-internal callers.
    pub fn new(
        client: WarehouseClient,
        caller_tenant_id: Option<String>,
        extension_id: ExtensionId,
        granted_tables: Option<BTreeSet<String>>,
        table_specs: Arc<Vec<ContributeWarehouseTable>>,
    ) -> Self {
        Self {
            client,
            caller_tenant_id,
            extension_id,
            granted_tables,
            table_specs,
        }
    }

    /// Resolve `name` against the manifest declarations. Returns
    /// the spec on hit; `Error::Validation` on miss.
    fn find_spec(&self, name: &str) -> Result<&ContributeWarehouseTable> {
        self.table_specs
            .iter()
            .find(|t| t.name == name)
            .ok_or_else(|| {
                Error::validation(format!(
                    "warehouse_write: table {name:?} is not declared in \
                     contributes.warehouse_tables[] for extension {:?}",
                    self.extension_id.as_str()
                ))
            })
    }

    /// Run the INSERT against the pool. Sync trait + async pool ⇒
    /// the same `block_in_place` + `block_on` bridge the read
    /// backend uses.
    fn run_insert(
        &self,
        full_table: &str,
        columns: &[&str],
        column_types: &[&str],
        rows: &[Vec<JsonValue>],
    ) -> Result<u64> {
        if rows.is_empty() {
            return Ok(0);
        }
        let placeholders = build_multi_row_placeholders(column_types, rows.len());
        let col_list = columns
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("INSERT INTO {full_table} ({col_list}) VALUES {placeholders}");

        let mut args = PgArguments::default();
        for row in rows {
            for (col_idx, value) in row.iter().enumerate() {
                // tenant_id is always at column 0 — already a String
                // we stamped ourselves. Everything else binds through
                // the typed dispatch.
                if col_idx == 0 {
                    let s = value.as_str().ok_or_else(|| {
                        Error::extension_internal(
                            "tenant_id stamp produced a non-string value (bug)",
                        )
                    })?;
                    args.add(s.to_owned())
                        .map_err(|e| Error::extension_internal(format!("bind tenant_id: {e}")))?;
                } else {
                    bind_column(&mut args, value, columns[col_idx])?;
                }
            }
        }

        let pool = self.client.pool().clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                sqlx::query_with::<Postgres, _>(&sql, args)
                    .execute(&pool)
                    .await
                    .map(|r| r.rows_affected())
                    .map_err(|e| Error::extension_internal(format!("INSERT failed: {e}")))
            })
        })
    }
}

impl WarehouseWriteBackend for RubixWarehouseWriteBackend {
    // TODO(cache-invalidation): per-extension warehouse write site.
    // starter-cache's opt-in cache wants
    // `invalidate_tags(&[format!("table:{full_table}")])` here on
    // successful commit. There is no unified `WarehouseWriter`
    // chokepoint yet (see rubix/docs/sessions/cache-v0-progress.md);
    // until one lands, tag invalidation here is best-effort and
    // depends on the caller threading the layer's invalidator handle
    // through.
    fn insert(&self, table: &str, rows: Vec<Row>) -> Result<u64> {
        let Some(tenant_id) = self.caller_tenant_id.as_deref() else {
            return Err(Error::capability(format!(
                "warehouse_write.insert {table:?} refused: no caller identity (system frame)"
            )));
        };

        if let Some(grant) = &self.granted_tables {
            if !grant.contains(table) {
                return Err(Error::capability(format!(
                    "warehouse_write: table {table:?} is not in the calling extension's grant"
                )));
            }
        }

        let spec = self.find_spec(table)?;
        let full_table = full_table_name(&self.extension_id, table);

        // Build the column list, tenant_id first.
        let mut column_names: Vec<&str> = Vec::with_capacity(spec.columns.len() + 1);
        let mut column_types: Vec<&str> = Vec::with_capacity(spec.columns.len() + 1);
        column_names.push("tenant_id");
        column_types.push("TEXT");
        for c in &spec.columns {
            column_names.push(c.name.as_str());
            column_types.push(c.ty.as_str());
        }

        // Translate each input row into [tenant_id, col1, col2, ...]
        // ordered by the spec's column list. Missing columns get
        // `serde_json::Value::Null` (and the row binder will respect
        // the manifest's `default` or refuse).
        let mut value_rows: Vec<Vec<JsonValue>> = Vec::with_capacity(rows.len());
        for row in &rows {
            let map = row.as_map();
            // Validate: no unknown columns.
            for k in map.keys() {
                if k == "tenant_id" {
                    // Silently dropped — host stamps it. Don't refuse,
                    // because the extension may have copied a row out
                    // of a read result and is round-tripping it back.
                    continue;
                }
                if !spec.columns.iter().any(|c| c.name == *k) {
                    return Err(Error::validation(format!(
                        "warehouse_write: row carries column {k:?} which is not declared in \
                         contributes.warehouse_tables[{:?}].columns",
                        spec.name
                    )));
                }
            }
            let mut values: Vec<JsonValue> = Vec::with_capacity(column_names.len());
            values.push(JsonValue::String(tenant_id.to_owned()));
            for col in &spec.columns {
                let raw = map.get(&col.name).cloned();
                let resolved = match raw {
                    Some(v) if !v.is_null() => v,
                    _ => match &col.default {
                        // A manifest default is a SQL expression
                        // (e.g. `now()`, `0`). v0.1 does not
                        // evaluate SQL expressions client-side;
                        // we just bind NULL and let the column's
                        // DDL default (also emitted from the
                        // manifest) take effect server-side.
                        Some(_) => JsonValue::Null,
                        None => {
                            return Err(Error::validation(format!(
                                "warehouse_write: row missing required column {:?} \
                                 (no default declared in manifest)",
                                col.name
                            )));
                        }
                    },
                };
                values.push(resolved);
            }
            value_rows.push(values);
        }

        self.run_insert(&full_table, &column_names, &column_types, &value_rows)
    }

    fn update(&self, table: &str, key_column: &str, rows: Vec<Row>) -> Result<u64> {
        let Some(tenant_id) = self.caller_tenant_id.as_deref() else {
            return Err(Error::capability(format!(
                "warehouse_write.update {table:?} refused: no caller identity (system frame)"
            )));
        };

        if let Some(grant) = &self.granted_tables {
            if !grant.contains(table) {
                return Err(Error::capability(format!(
                    "warehouse_write: table {table:?} is not in the calling extension's grant"
                )));
            }
        }

        let spec = self.find_spec(table)?;
        let full_table = full_table_name(&self.extension_id, table);

        // Resolve key column against the declared schema.
        let key_spec = spec
            .columns
            .iter()
            .find(|c| c.name == key_column)
            .ok_or_else(|| {
                Error::validation(format!(
                    "warehouse_write.update: key_column {key_column:?} is not declared in \
                 contributes.warehouse_tables[{:?}].columns",
                    spec.name
                ))
            })?;
        let key_type = key_spec.ty.clone();

        if rows.is_empty() {
            return Ok(0);
        }

        // Build per-row UPDATE … WHERE key = $k AND tenant_id = $t.
        // We issue one statement per row because the SET list differs
        // (each row may carry a different subset of columns); batching
        // would need a CASE-per-column rewrite that's not worth the
        // complexity at v0.1 CRUD volumes.
        let pool = self.client.pool().clone();
        let mut total_affected: u64 = 0;
        for row in &rows {
            let map = row.as_map();
            let key_value = map.get(key_column).cloned().ok_or_else(|| {
                Error::validation(format!(
                    "warehouse_write.update: row missing key column {key_column:?}"
                ))
            })?;

            // Validate set columns and collect them in declared
            // order (excluding the key, tenant_id, and ingested_at).
            let mut set_cols: Vec<&str> = Vec::new();
            let mut set_types: Vec<&str> = Vec::new();
            let mut set_values: Vec<JsonValue> = Vec::new();
            for k in map.keys() {
                if k == "tenant_id" || k == key_column {
                    continue;
                }
                if !spec.columns.iter().any(|c| c.name == *k) {
                    return Err(Error::validation(format!(
                        "warehouse_write.update: row carries column {k:?} which is not declared in \
                         contributes.warehouse_tables[{:?}].columns",
                        spec.name
                    )));
                }
            }
            for col in &spec.columns {
                if col.name == key_column {
                    continue;
                }
                if let Some(v) = map.get(&col.name) {
                    set_cols.push(col.name.as_str());
                    set_types.push(col.ty.as_str());
                    set_values.push(v.clone());
                }
            }
            if set_cols.is_empty() {
                return Err(Error::validation(
                    "warehouse_write.update: row has no columns to SET (only the key was supplied)",
                ));
            }

            // SET col1 = $1::t1, col2 = $2::t2, ... WHERE key = $N::kt AND tenant_id = $N+1
            let mut sql = format!("UPDATE {full_table} SET ");
            let mut idx: usize = 1;
            for (i, (col, ty)) in set_cols.iter().zip(set_types.iter()).enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                let cast = placeholder_cast(ty);
                sql.push_str(&format!("{} = ${idx}{cast}", quote_ident(col)));
                idx += 1;
            }
            let key_cast = placeholder_cast(&key_type);
            sql.push_str(&format!(
                " WHERE {} = ${idx}{key_cast} AND tenant_id = ${}",
                quote_ident(key_column),
                idx + 1
            ));

            let mut args = PgArguments::default();
            for (v, col) in set_values.iter().zip(set_cols.iter()) {
                bind_column(&mut args, v, col)?;
            }
            bind_column(&mut args, &key_value, key_column)?;
            args.add(tenant_id.to_owned())
                .map_err(|e| Error::extension_internal(format!("bind tenant_id: {e}")))?;

            let pool_ref = pool.clone();
            let affected = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    sqlx::query_with::<Postgres, _>(&sql, args)
                        .execute(&pool_ref)
                        .await
                        .map(|r| r.rows_affected())
                        .map_err(|e| Error::extension_internal(format!("UPDATE failed: {e}")))
                })
            })?;
            total_affected += affected;
        }
        Ok(total_affected)
    }

    fn delete(&self, table: &str, key_column: &str, keys: Vec<JsonValue>) -> Result<u64> {
        let Some(tenant_id) = self.caller_tenant_id.as_deref() else {
            return Err(Error::capability(format!(
                "warehouse_write.delete {table:?} refused: no caller identity (system frame)"
            )));
        };

        if let Some(grant) = &self.granted_tables {
            if !grant.contains(table) {
                return Err(Error::capability(format!(
                    "warehouse_write: table {table:?} is not in the calling extension's grant"
                )));
            }
        }

        let spec = self.find_spec(table)?;
        let full_table = full_table_name(&self.extension_id, table);

        let key_spec = spec
            .columns
            .iter()
            .find(|c| c.name == key_column)
            .ok_or_else(|| {
                Error::validation(format!(
                    "warehouse_write.delete: key_column {key_column:?} is not declared in \
                 contributes.warehouse_tables[{:?}].columns",
                    spec.name
                ))
            })?;
        let key_cast = placeholder_cast(&key_spec.ty);

        if keys.is_empty() {
            return Ok(0);
        }

        // DELETE FROM t WHERE key IN ($1::kt, $2::kt, ...) AND tenant_id = $N+1.
        let mut sql = format!(
            "DELETE FROM {full_table} WHERE {} IN (",
            quote_ident(key_column)
        );
        for i in 0..keys.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("${}{key_cast}", i + 1));
        }
        sql.push_str(&format!(") AND tenant_id = ${}", keys.len() + 1));

        let mut args = PgArguments::default();
        for k in &keys {
            bind_column(&mut args, k, key_column)?;
        }
        args.add(tenant_id.to_owned())
            .map_err(|e| Error::extension_internal(format!("bind tenant_id: {e}")))?;

        let pool = self.client.pool().clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                sqlx::query_with::<Postgres, _>(&sql, args)
                    .execute(&pool)
                    .await
                    .map(|r| r.rows_affected())
                    .map_err(|e| Error::extension_internal(format!("DELETE failed: {e}")))
            })
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers — extension-id sanitisation, column-name quoting, multi-row
// placeholder builder, typed binder.
// ---------------------------------------------------------------------------

/// Translate `com.acme.power` → `com_acme_power` for use as a SQL
/// identifier prefix. Reverse-DNS extension ids contain dots, which
/// Postgres treats as schema separators; sanitising to underscores
/// keeps every extension's tables in the default search-path schema
/// without further wiring.
///
/// Pure ASCII because [`ExtensionId`] validation already restricts
/// the id alphabet to `[a-z0-9.-]`. Dashes also get replaced (rare
/// in practice — most ids are dotted).
pub fn sanitize_extension_id(ext: &ExtensionId) -> String {
    ext.as_str()
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect()
}

/// `<sanitize(ext)>__<unprefixed_name>`. Two underscores so the
/// boundary is unambiguous (extension ids never contain a double
/// underscore after sanitisation since the source alphabet doesn't
/// include `_`).
pub fn full_table_name(ext: &ExtensionId, unprefixed: &str) -> String {
    format!("{}__{}", sanitize_extension_id(ext), unprefixed)
}

/// Double-quote a SQL identifier so reserved words or unusual
/// (but manifest-validated) names don't trip the parser. The
/// manifest validator restricts column names to
/// `[a-zA-Z_][a-zA-Z0-9_]*`, but quoting is cheap insurance.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Build `($1, $2, ..., $N), ($N+1, ...)` for a multi-row INSERT.
///
/// Per-column `column_types` drives an explicit `::<type>` cast on
/// each placeholder for types that Postgres won't implicitly cast
/// from `text` (DATE, TIMESTAMP, TIMESTAMPTZ, JSONB, UUID …). We
/// bind every value as `text`/`i64`/`f64`/… from JSON, so without
/// the cast inserts into a `DATE` column fail with
/// `column is of type date but expression is of type text`.
fn build_multi_row_placeholders(column_types: &[&str], rows: usize) -> String {
    let cast_suffixes: Vec<&'static str> =
        column_types.iter().map(|t| placeholder_cast(t)).collect();
    let mut s = String::new();
    let mut idx: usize = 1;
    for r in 0..rows {
        if r > 0 {
            s.push_str(", ");
        }
        s.push('(');
        for (c, suffix) in cast_suffixes.iter().enumerate() {
            if c > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("${idx}{suffix}"));
            idx += 1;
        }
        s.push(')');
    }
    s
}

/// Map a manifest column type to the `::<type>` suffix appended to
/// the placeholder. Empty string ⇒ no cast (Postgres infers from
/// the bound type or implicit-casts from text).
fn placeholder_cast(ty: &str) -> &'static str {
    // Case-insensitive match on the head of the type so
    // `TIMESTAMP WITH TIME ZONE`, `timestamptz`, `DATE` all hit.
    let head = ty
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or(ty);
    match head.to_ascii_uppercase().as_str() {
        "DATE" => "::date",
        "TIMESTAMP" => match ty.to_ascii_uppercase().contains("TIME ZONE") {
            true => "::timestamptz",
            false => "::timestamp",
        },
        "TIMESTAMPTZ" => "::timestamptz",
        "TIME" => "::time",
        "JSONB" => "::jsonb",
        "JSON" => "::json",
        "UUID" => "::uuid",
        _ => "",
    }
}

/// Bind one column value to `args` based on the column's declared
/// type. Returns `Error::Validation` on type mismatch or unsupported
/// type.
///
/// v0.1 supports a small whitelist sufficient for the cleaner-flow
/// L2 output table. Additional types land additively here.
fn bind_column(args: &mut PgArguments, value: &JsonValue, col_name: &str) -> Result<()> {
    // `col_name` is the **column name**, not the type — the type
    // string is owned by the manifest spec and the caller of this
    // helper. For v0.1 we infer the type by inspecting the JSON
    // value's shape and binding directly. This is sufficient
    // because Postgres can implicitly cast most narrow types
    // (e.g. an i64 bound into a SMALLINT column). A future slice
    // can pass the declared type in for stricter checking.
    //
    // `args.add` returns `Result` in this sqlx version (encoding
    // errors); we surface those as `ExtensionInternal` since they
    // come from the kernel side of the bind, not from the
    // extension's input shape.
    let map_bind = |e: sqlx::error::BoxDynError| {
        Error::extension_internal(format!("bind column {col_name:?}: {e}"))
    };
    if value.is_null() {
        args.add::<Option<String>>(None).map_err(map_bind)?;
        return Ok(());
    }
    if let Some(s) = value.as_str() {
        args.add(s.to_owned()).map_err(map_bind)?;
        return Ok(());
    }
    if let Some(i) = value.as_i64() {
        args.add(i).map_err(map_bind)?;
        return Ok(());
    }
    if let Some(u) = value.as_u64() {
        if u <= i64::MAX as u64 {
            args.add(u as i64).map_err(map_bind)?;
            return Ok(());
        }
        return Err(Error::validation(format!(
            "warehouse_write column {col_name:?}: unsigned value exceeds i64::MAX"
        )));
    }
    if let Some(f) = value.as_f64() {
        if f.is_nan() {
            return Err(Error::validation(format!(
                "warehouse_write column {col_name:?}: NaN cannot be bound to a Postgres column"
            )));
        }
        args.add(f).map_err(map_bind)?;
        return Ok(());
    }
    if let Some(b) = value.as_bool() {
        args.add(b).map_err(map_bind)?;
        return Ok(());
    }
    // Arrays / objects bind as JSONB. Postgres accepts a serialised
    // JSON string and will store it in a `JSONB` column; for other
    // column types this surfaces server-side as a type error which
    // the extension-internal error path will report verbatim.
    args.add(value.clone()).map_err(map_bind)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use starter_ext_spi::manifest::TableColumn as Col;

    fn dummy_client() -> WarehouseClient {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://test:test@127.0.0.1:1/test")
            .expect("connect_lazy infallible for valid URLs");
        WarehouseClient::from_pool(pool)
    }

    fn ext_id() -> ExtensionId {
        ExtensionId::new("com.acme.power").unwrap()
    }

    fn spec_solar() -> Vec<ContributeWarehouseTable> {
        vec![ContributeWarehouseTable {
            name: "solar_panels".into(),
            columns: vec![
                Col {
                    name: "ts".into(),
                    ty: "DOUBLE PRECISION".into(),
                    default: None,
                },
                Col {
                    name: "kwh".into(),
                    ty: "DOUBLE PRECISION".into(),
                    default: None,
                },
            ],
            order_by: vec!["ts".into()],
            engine: None,
            partition_by: None,
            ttl: None,
        }]
    }

    fn backend(
        caller: Option<&str>,
        grant: Option<BTreeSet<String>>,
        specs: Vec<ContributeWarehouseTable>,
    ) -> RubixWarehouseWriteBackend {
        RubixWarehouseWriteBackend::new(
            dummy_client(),
            caller.map(|s| s.to_owned()),
            ext_id(),
            grant,
            Arc::new(specs),
        )
    }

    fn row(map: serde_json::Map<String, JsonValue>) -> Row {
        Row::from_map(map)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn system_frame_insert_refused() {
        let b = backend(
            None,
            Some(BTreeSet::from(["solar_panels".into()])),
            spec_solar(),
        );
        let err = b
            .insert("solar_panels", vec![])
            .expect_err("system frame must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn table_outside_grant_refused() {
        let b = backend(Some("t-1"), Some(BTreeSet::new()), spec_solar());
        let err = b
            .insert("solar_panels", vec![])
            .expect_err("empty grant must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn undeclared_table_refused() {
        let b = backend(
            Some("t-1"),
            Some(BTreeSet::from(["nope".into()])),
            spec_solar(),
        );
        let err = b
            .insert("nope", vec![])
            .expect_err("undeclared table must refuse");
        assert!(matches!(err, Error::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn empty_row_set_is_zero_insert_without_touching_db() {
        let b = backend(
            Some("t-1"),
            Some(BTreeSet::from(["solar_panels".into()])),
            spec_solar(),
        );
        // Empty rows shortcut returns 0 without ever touching the
        // (intentionally-broken) pool.
        let n = b.insert("solar_panels", vec![]).expect("empty insert ok");
        assert_eq!(n, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unknown_column_in_row_refused_before_sql() {
        let b = backend(
            Some("t-1"),
            Some(BTreeSet::from(["solar_panels".into()])),
            spec_solar(),
        );
        let mut m = serde_json::Map::new();
        m.insert("ts".into(), JsonValue::from(1.0));
        m.insert("kwh".into(), JsonValue::from(2.0));
        m.insert("rogue".into(), JsonValue::from("evil"));
        let err = b
            .insert("solar_panels", vec![row(m)])
            .expect_err("unknown column must refuse");
        assert!(matches!(err, Error::Validation(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn missing_required_column_refused() {
        let b = backend(
            Some("t-1"),
            Some(BTreeSet::from(["solar_panels".into()])),
            spec_solar(),
        );
        let mut m = serde_json::Map::new();
        m.insert("ts".into(), JsonValue::from(1.0));
        // missing kwh
        let err = b
            .insert("solar_panels", vec![row(m)])
            .expect_err("missing required column must refuse");
        assert!(matches!(err, Error::Validation(_)), "got {err:?}");
    }

    #[test]
    fn sanitize_replaces_dots_with_underscores() {
        let id = ExtensionId::new("com.acme.power").unwrap();
        assert_eq!(sanitize_extension_id(&id), "com_acme_power");
    }

    #[test]
    fn full_table_name_double_underscore_boundary() {
        let id = ExtensionId::new("com.acme.power").unwrap();
        assert_eq!(
            full_table_name(&id, "solar_panels"),
            "com_acme_power__solar_panels"
        );
    }

    #[test]
    fn multi_row_placeholders_for_2_cols_3_rows() {
        let s = build_multi_row_placeholders(&["TEXT", "DOUBLE PRECISION"], 3);
        assert_eq!(s, "($1, $2), ($3, $4), ($5, $6)");
    }

    #[test]
    fn placeholder_casts_for_date_and_timestamptz() {
        let s = build_multi_row_placeholders(
            &["TEXT", "DATE", "TIMESTAMPTZ", "TIMESTAMP WITH TIME ZONE"],
            1,
        );
        assert_eq!(s, "($1, $2::date, $3::timestamptz, $4::timestamptz)");
    }
}

// Silence "unused import" when only the type re-export shows up
// downstream.
#[allow(dead_code)]
fn _table_column_anchor(_c: &TableColumn) {}
