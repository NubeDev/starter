//! WS-17 Wave A — extension-owned tables in the nexus Postgres DB.
//!
//! An extension declares tables in `contributes.warehouse_tables[]`; nexus
//! creates them at boot as `<sanitized_ext_id>__<name>` (the extension-id
//! prefix is the ownership + isolation boundary) and exposes a tenant-stamped
//! `warehouse.write`/`warehouse.update`/`warehouse.delete` host method over
//! them. This is the rubix `extension_tables.rs` + `warehouse_write.rs` model,
//! ported to **plain Postgres** (the nexus `metadata` pool) — no ClickHouse, no
//! `WarehouseClient`, no background-index single-flight (nexus tables are
//! freshly created and empty, so the `order_by` index builds inline).
//!
//! Three concerns live here:
//! - [`full_table_name`] / [`sanitize_extension_id`] — the ownership-prefix
//!   naming, identical to rubix so a ported manifest lands the same table name.
//! - [`create_extension_tables`] — boot-time `CREATE TABLE IF NOT EXISTS` from
//!   each enabled extension's `warehouse_tables[]`, `tenant_id` prepended,
//!   Postgres-typed verbatim. Idempotent (re-boot is a no-op).
//! - [`WriteExecutor`] — the per-call INSERT/UPSERT/UPDATE/DELETE the host
//!   methods route to, with the own-table allowlist, the tenant clamp, and
//!   column validation against the manifest spec.

use std::collections::BTreeSet;

use serde_json::Value as JsonValue;
use sqlx::{postgres::PgArguments, Arguments, Executor, PgPool, Postgres};
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::manifest::{ContributeWarehouseTable, TableColumn};
use starter_ext_spi::warehouse::Row;
use starter_ext_spi::{Error as ExtError, ExtensionId, LifecycleState, Result as ExtResult};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Naming — the ownership prefix (ported verbatim from rubix so a manifest
// authored against either host resolves to the same table name).
// ---------------------------------------------------------------------------

/// Translate `com.acme.devices` → `com_acme_devices` for use as a SQL
/// identifier prefix. Reverse-DNS extension ids contain dots, which Postgres
/// treats as schema separators; sanitising to underscores keeps every
/// extension's tables in the default search-path schema. Dashes are replaced
/// too. `ExtensionId` validation already restricts the alphabet to
/// `[a-z0-9.-]`, so the result is always a safe identifier head.
pub fn sanitize_extension_id(ext: &ExtensionId) -> String {
    ext.as_str()
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect()
}

/// `<sanitize(ext)>__<unprefixed_name>`. Two underscores so the boundary is
/// unambiguous (a sanitised id never contains a double underscore — the source
/// alphabet has no `_`).
pub fn full_table_name(ext: &ExtensionId, unprefixed: &str) -> String {
    format!("{}__{}", sanitize_extension_id(ext), unprefixed)
}

/// Double-quote a SQL identifier. Names are manifest-validated to
/// `[a-zA-Z_][a-zA-Z0-9_]*`, but quoting is cheap insurance against reserved
/// words.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Validate that `s` is a safe SQL identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
fn validate_identifier(s: &str, kind: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("{kind} name is empty"));
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!(
            "{kind} name {s:?} must start with a letter or underscore"
        ));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "{kind} name {s:?} contains character {c:?} (allowed: [a-zA-Z0-9_])"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Boot-time DDL.
// ---------------------------------------------------------------------------

/// Outcome summary from one boot-DDL sweep, logged as a single line so an
/// operator can spot drift.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExtensionTablesOutcome {
    /// Tables seen across every validated extension.
    pub seen: usize,
    /// Tables for which `CREATE TABLE IF NOT EXISTS` succeeded.
    pub created_or_existing: usize,
    /// Tables skipped for a non-fatal reason (invalid name, `tenant_id`
    /// collision, DDL failure). One warn line per skip names the offender.
    pub skipped: usize,
    /// Entries the host intentionally did not create because their `kind` opts
    /// out of host-managed DDL (continuous aggregates owned by the extension).
    pub deferred_to_extension: usize,
}

/// Walk the sealed registry and create every validated extension's declared
/// `warehouse_tables[]` against `metadata` (the nexus Postgres). The companion
/// `order_by` index is created inline — unlike rubix, nexus tables are freshly
/// created and empty at boot, so there is no multi-hour `CREATE INDEX` to
/// background. Per-table failure logs at warn and continues; one bad manifest
/// cannot block the host from coming up.
pub async fn create_extension_tables(
    metadata: &PgPool,
    registry: &ExtensionRegistry,
) -> ExtensionTablesOutcome {
    let mut outcome = ExtensionTablesOutcome::default();
    for record in registry.iter_validated() {
        if record.state != LifecycleState::Validated {
            continue;
        }
        let (Some(ext_id), Some(manifest)) = (record.id.as_ref(), record.manifest.as_ref()) else {
            continue;
        };
        for entry in &manifest.contributes.warehouse_tables {
            outcome.seen += 1;
            if !entry.kind.host_manages_ddl() {
                // CAGG / extension-managed relation: the entry stays registered
                // (so the write allowlist still authorises it) but creation is
                // the extension's job — emitting a plain table here would race a
                // materialised view and leave an empty stub.
                outcome.deferred_to_extension += 1;
                info!(
                    target: "nexus_api::extensions::warehouse",
                    extension = %ext_id.as_str(),
                    table = %entry.name,
                    kind = ?entry.kind,
                    "deferring extension-table DDL (non-table kind)"
                );
                continue;
            }
            match apply_table(metadata, ext_id, entry).await {
                Ok(()) => outcome.created_or_existing += 1,
                Err(reason) => {
                    outcome.skipped += 1;
                    warn!(
                        target: "nexus_api::extensions::warehouse",
                        extension = %ext_id.as_str(),
                        table = %entry.name,
                        reason = %reason,
                        "skipping extension-table DDL"
                    );
                }
            }
        }
    }
    info!(
        target: "nexus_api::extensions::warehouse",
        seen = outcome.seen,
        created_or_existing = outcome.created_or_existing,
        skipped = outcome.skipped,
        deferred_to_extension = outcome.deferred_to_extension,
        "extension warehouse-table DDL applied"
    );
    outcome
}

/// Create one declared table (validate → `CREATE TABLE IF NOT EXISTS` → index).
/// The boot sweep ([`create_extension_tables`]) calls this per entry; exposed so
/// tests and a future install-time hook can create a single table without a
/// full registry. Returns the validation/DDL error string on failure.
pub async fn create_one_table(
    metadata: &PgPool,
    ext_id: &ExtensionId,
    entry: &ContributeWarehouseTable,
) -> Result<(), String> {
    apply_table(metadata, ext_id, entry).await
}

/// Validate one entry, `CREATE TABLE IF NOT EXISTS` it, then create its
/// `order_by` index.
async fn apply_table(
    metadata: &PgPool,
    ext_id: &ExtensionId,
    entry: &ContributeWarehouseTable,
) -> Result<(), String> {
    validate_table_entry(entry)?;
    let full_table = full_table_name(ext_id, &entry.name);

    let create_sql = build_create_sql(&full_table, entry);
    metadata
        .execute(create_sql.as_str())
        .await
        .map_err(|e| format!("CREATE TABLE: {e}"))?;

    if let Some(index_sql) = build_index_sql(ext_id, &full_table, entry) {
        metadata
            .execute(index_sql.as_str())
            .await
            .map_err(|e| format!("CREATE INDEX: {e}"))?;
    }

    // Deployments that split the DDL/owner role from the non-BYPASSRLS runtime
    // role (the role the `metadata` pool connects as to serve `warehouse.write`)
    // must grant the runtime role CRUD on the freshly-created table, or every
    // write would fail with "permission denied". Single-role deployments (dev,
    // the common case — migrations and runtime share one DSN) leave
    // `NEXUS_RUNTIME_ROLE` unset and skip this; the owner already has access.
    // The table carries no RLS policy, so tenant isolation is enforced by the
    // `tenant_id` predicate in every query/write, not by row security.
    if let Ok(role) = std::env::var("NEXUS_RUNTIME_ROLE") {
        let role = role.trim();
        if !role.is_empty() {
            // Role name is operator-supplied config; quote it as an identifier.
            let grant_sql = format!(
                "GRANT SELECT, INSERT, UPDATE, DELETE ON {} TO {}",
                quote_ident(&full_table),
                quote_ident(role)
            );
            metadata
                .execute(grant_sql.as_str())
                .await
                .map_err(|e| format!("GRANT to runtime role {role:?}: {e}"))?;
        }
    }
    Ok(())
}

/// Validate the manifest entry's names + `order_by` references. Shared by boot
/// DDL and (defensively) by the write path's spec lookups.
fn validate_table_entry(entry: &ContributeWarehouseTable) -> Result<(), String> {
    validate_identifier(&entry.name, "table")?;
    for col in &entry.columns {
        validate_identifier(&col.name, "column")?;
        if col.name == "tenant_id" {
            return Err("column \"tenant_id\" is reserved (host stamps it on every write)".into());
        }
    }
    for ob in &entry.order_by {
        // `order_by` may reference the host-prepended `tenant_id`.
        if ob != "tenant_id" && !entry.columns.iter().any(|c| &c.name == ob) {
            return Err(format!(
                "order_by references column {ob:?} which is not in `columns`"
            ));
        }
    }
    Ok(())
}

/// `CREATE TABLE IF NOT EXISTS "<full_table>" (tenant_id, <cols…>[, PRIMARY KEY …])`.
///
/// When `order_by` is non-empty it becomes a `PRIMARY KEY (tenant_id,
/// order_by…)`. The PK is what makes the upsert path (`ON CONFLICT`) work and
/// gives idempotent `device_create` semantics (WS-17 §4.1.2 / Q4: `order_by` is
/// the conflict target). Column types are Postgres types, used verbatim
/// (WS-17 Q1).
fn build_create_sql(full_table: &str, entry: &ContributeWarehouseTable) -> String {
    let mut s = format!("CREATE TABLE IF NOT EXISTS {} (\n", quote_ident(full_table));
    s.push_str("    \"tenant_id\" TEXT NOT NULL");
    for col in &entry.columns {
        s.push_str(",\n    ");
        s.push_str(&format_column(col));
    }
    if !entry.order_by.is_empty() {
        let pk_cols = std::iter::once("tenant_id")
            .chain(entry.order_by.iter().map(String::as_str))
            .map(quote_ident)
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!(",\n    PRIMARY KEY ({pk_cols})"));
    }
    s.push_str("\n)");
    s
}

fn format_column(col: &TableColumn) -> String {
    let name = quote_ident(&col.name);
    match &col.default {
        Some(expr) => format!("{name} {} DEFAULT {expr}", col.ty),
        None => format!("{name} {}", col.ty),
    }
}

/// The `order_by` covering index, or `None` if no `order_by`. The PRIMARY KEY
/// already indexes `(tenant_id, order_by…)`, so this is only emitted when the
/// table has **no** PK to lean on — kept for parity with rubix and future
/// non-PK (append-only) tables. Today every `order_by` table gets a PK, so this
/// returns `None`; an `order_by`-less table gets neither.
fn build_index_sql(
    _ext_id: &ExtensionId,
    _full_table: &str,
    _entry: &ContributeWarehouseTable,
) -> Option<String> {
    // The PRIMARY KEY built in `build_create_sql` is the (tenant_id, order_by…)
    // index, so a separate index would be redundant. Reserved as a seam for
    // append-only tables (no PK) if they land later.
    None
}

// ---------------------------------------------------------------------------
// Write path — per-call INSERT/UPSERT/UPDATE/DELETE.
// ---------------------------------------------------------------------------

/// Per-call write executor: resolves the calling extension's manifest spec for
/// `table`, clamps the tenant, validates columns, and runs the statement
/// against `metadata`. Built fresh per host-method call (cheap — borrows).
pub struct WriteExecutor<'a> {
    metadata: &'a PgPool,
    extension_id: &'a ExtensionId,
    tenant_id: &'a str,
    /// The calling extension's declared tables, from
    /// `contributes.warehouse_tables[]`. The per-call own-table allowlist: a
    /// table not declared here is refused (an extension cannot write another's
    /// table or an arbitrary nexus table).
    specs: &'a [ContributeWarehouseTable],
    /// Unprefixed table names the extension's `warehouse_write` grant permits
    /// (`capabilities.warehouse_write.tables`). `None` means an empty/absent
    /// grant ⇒ refuse every table.
    granted: Option<&'a BTreeSet<String>>,
}

impl<'a> WriteExecutor<'a> {
    pub fn new(
        metadata: &'a PgPool,
        extension_id: &'a ExtensionId,
        tenant_id: &'a str,
        specs: &'a [ContributeWarehouseTable],
        granted: Option<&'a BTreeSet<String>>,
    ) -> Self {
        Self {
            metadata,
            extension_id,
            tenant_id,
            specs,
            granted,
        }
    }

    /// Gate `table` against the grant + manifest declarations and return its
    /// spec. The grant gate runs first (capability boundary), then the
    /// declaration lookup (which table schema to bind against).
    fn resolve(&self, table: &str) -> ExtResult<&'a ContributeWarehouseTable> {
        match self.granted {
            Some(grant) if grant.contains(table) => {}
            _ => {
                return Err(ExtError::capability(format!(
                    "warehouse.write: table {table:?} is not in extension {:?}'s \
                     warehouse_write grant",
                    self.extension_id.as_str()
                )))
            }
        }
        self.specs
            .iter()
            .find(|t| t.name == table)
            .ok_or_else(|| {
                ExtError::validation(format!(
                    "warehouse.write: table {table:?} is not declared in \
                     contributes.warehouse_tables[] for extension {:?}",
                    self.extension_id.as_str()
                ))
            })
    }

    /// Insert `rows` into `table`, tenant-stamped. When the table declares an
    /// `order_by` (its PRIMARY KEY conflict target), the insert is an **upsert**
    /// (`ON CONFLICT … DO UPDATE`) so a re-run with the same natural key updates
    /// rather than erroring — the idempotent-node contract (WS-17 §4.1.2 / Q4).
    pub async fn insert(&self, table: &str, rows: Vec<Row>) -> ExtResult<u64> {
        let spec = self.resolve(table)?;
        if rows.is_empty() {
            return Ok(0);
        }
        let full_table = full_table_name(self.extension_id, table);

        // Reject unknown columns up front.
        for row in &rows {
            for k in row.as_map().keys() {
                if k == "tenant_id" {
                    // Dropped silently — the host stamps it. Don't refuse, the
                    // extension may be round-tripping a read result.
                    continue;
                }
                if !spec.columns.iter().any(|c| c.name == *k) {
                    return Err(ExtError::validation(format!(
                        "warehouse.write: row carries column {k:?} not declared in \
                         contributes.warehouse_tables[{:?}].columns",
                        spec.name
                    )));
                }
            }
        }

        // Choose which declared columns appear in the INSERT. A column is
        // included if any row supplies a non-null value for it, OR it has no
        // manifest `default` (required — it must be present in every row). A
        // column absent from every row that DOES have a default is **omitted**
        // entirely, so the column's DDL `DEFAULT` fires server-side — binding an
        // explicit NULL would defeat the default (and break a NOT NULL column).
        let included: Vec<&TableColumn> = spec
            .columns
            .iter()
            .filter(|c| {
                c.default.is_none()
                    || rows
                        .iter()
                        .any(|r| r.as_map().get(&c.name).is_some_and(|v| !v.is_null()))
            })
            .collect();

        // tenant_id first, then the included declared columns.
        let mut column_names: Vec<&str> = Vec::with_capacity(included.len() + 1);
        let mut column_types: Vec<&str> = Vec::with_capacity(included.len() + 1);
        column_names.push("tenant_id");
        column_types.push("TEXT");
        for c in &included {
            column_names.push(c.name.as_str());
            column_types.push(c.ty.as_str());
        }

        // Build [tenant_id, included_cols…] per row. A required column missing
        // from a row is refused; a defaulted-but-included column missing from a
        // row binds NULL (it is included only because another row supplied it,
        // so this row genuinely wants NULL there).
        let mut value_rows: Vec<Vec<JsonValue>> = Vec::with_capacity(rows.len());
        for row in &rows {
            let map = row.as_map();
            let mut values: Vec<JsonValue> = Vec::with_capacity(column_names.len());
            values.push(JsonValue::String(self.tenant_id.to_owned()));
            for col in &included {
                let raw = map.get(&col.name).cloned();
                let resolved = match raw {
                    Some(v) if !v.is_null() => v,
                    _ if col.default.is_none() => {
                        return Err(ExtError::validation(format!(
                            "warehouse.write: row missing required column {:?} \
                             (no default declared in manifest)",
                            col.name
                        )))
                    }
                    _ => JsonValue::Null,
                };
                values.push(resolved);
            }
            value_rows.push(values);
        }

        let placeholders = build_multi_row_placeholders(&column_types, value_rows.len());
        let col_list = column_names
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        let mut sql = format!(
            "INSERT INTO {} ({col_list}) VALUES {placeholders}",
            quote_ident(&full_table)
        );
        // Upsert on the PK (tenant_id + order_by) when declared.
        if !spec.order_by.is_empty() {
            let conflict_cols = std::iter::once("tenant_id")
                .chain(spec.order_by.iter().map(String::as_str))
                .map(quote_ident)
                .collect::<Vec<_>>()
                .join(", ");
            // Overwrite every non-key INCLUDED column with the excluded (new)
            // value, so a repeat key is an update. Only columns actually in the
            // INSERT list can be referenced via EXCLUDED — a defaulted column we
            // omitted keeps its existing value on conflict (we have no new value
            // to set it to anyway).
            let set_clause = included
                .iter()
                .filter(|c| !spec.order_by.iter().any(|ob| ob == &c.name))
                .map(|c| format!("{0} = EXCLUDED.{0}", quote_ident(&c.name)))
                .collect::<Vec<_>>()
                .join(", ");
            if set_clause.is_empty() {
                // Every column is part of the key — nothing to update; a repeat
                // is a no-op rather than an error.
                sql.push_str(&format!(" ON CONFLICT ({conflict_cols}) DO NOTHING"));
            } else {
                sql.push_str(&format!(
                    " ON CONFLICT ({conflict_cols}) DO UPDATE SET {set_clause}"
                ));
            }
        }

        let mut args = PgArguments::default();
        for row in &value_rows {
            for (col_idx, value) in row.iter().enumerate() {
                if col_idx == 0 {
                    // tenant_id — a String we stamped ourselves.
                    let s = value.as_str().ok_or_else(|| {
                        ExtError::extension_internal("tenant_id stamp produced a non-string (bug)")
                    })?;
                    args.add(s.to_owned())
                        .map_err(|e| ExtError::extension_internal(format!("bind tenant_id: {e}")))?;
                } else {
                    bind_column(&mut args, value, column_names[col_idx])?;
                }
            }
        }

        sqlx::query_with::<Postgres, _>(&sql, args)
            .execute(self.metadata)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| ExtError::extension_internal(format!("warehouse.write INSERT: {e}")))
    }

    /// Update `rows` in `table`, matching each by `key_column = value AND
    /// tenant_id = caller`. One statement per row (the SET list differs per
    /// row). Columns absent from a row are left unchanged.
    pub async fn update(&self, table: &str, key_column: &str, rows: Vec<Row>) -> ExtResult<u64> {
        let spec = self.resolve(table)?;
        let key_spec = spec
            .columns
            .iter()
            .find(|c| c.name == key_column)
            .ok_or_else(|| {
                ExtError::validation(format!(
                    "warehouse.update: key_column {key_column:?} is not declared in \
                     contributes.warehouse_tables[{:?}].columns",
                    spec.name
                ))
            })?;
        let key_type = key_spec.ty.clone();
        if rows.is_empty() {
            return Ok(0);
        }
        let full_table = full_table_name(self.extension_id, table);

        let mut total: u64 = 0;
        for row in &rows {
            let map = row.as_map();
            let key_value = map.get(key_column).cloned().ok_or_else(|| {
                ExtError::validation(format!(
                    "warehouse.update: row missing key column {key_column:?}"
                ))
            })?;
            for k in map.keys() {
                if k == "tenant_id" || k == key_column {
                    continue;
                }
                if !spec.columns.iter().any(|c| c.name == *k) {
                    return Err(ExtError::validation(format!(
                        "warehouse.update: row carries column {k:?} not declared in \
                         contributes.warehouse_tables[{:?}].columns",
                        spec.name
                    )));
                }
            }
            let mut set_cols: Vec<&str> = Vec::new();
            let mut set_values: Vec<JsonValue> = Vec::new();
            for col in &spec.columns {
                if col.name == key_column {
                    continue;
                }
                if let Some(v) = map.get(&col.name) {
                    set_cols.push(col.name.as_str());
                    set_values.push(v.clone());
                }
            }
            if set_cols.is_empty() {
                return Err(ExtError::validation(
                    "warehouse.update: row has no columns to SET (only the key was supplied)",
                ));
            }
            let mut sql = format!("UPDATE {} SET ", quote_ident(&full_table));
            let mut idx: usize = 1;
            for (i, col) in set_cols.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                let cast = placeholder_cast(spec.columns.iter().find(|c| &c.name == col).unwrap().ty.as_str());
                sql.push_str(&format!("{} = ${idx}{cast}", quote_ident(col)));
                idx += 1;
            }
            let key_cast = placeholder_cast(&key_type);
            sql.push_str(&format!(
                " WHERE {} = ${idx}{key_cast} AND \"tenant_id\" = ${}",
                quote_ident(key_column),
                idx + 1
            ));

            let mut args = PgArguments::default();
            for (v, col) in set_values.iter().zip(set_cols.iter()) {
                bind_column(&mut args, v, col)?;
            }
            bind_column(&mut args, &key_value, key_column)?;
            args.add(self.tenant_id.to_owned())
                .map_err(|e| ExtError::extension_internal(format!("bind tenant_id: {e}")))?;

            let affected = sqlx::query_with::<Postgres, _>(&sql, args)
                .execute(self.metadata)
                .await
                .map(|r| r.rows_affected())
                .map_err(|e| ExtError::extension_internal(format!("warehouse.update UPDATE: {e}")))?;
            total += affected;
        }
        Ok(total)
    }

    /// Delete rows from `table` where `key_column IN (keys) AND tenant_id =
    /// caller`.
    pub async fn delete(
        &self,
        table: &str,
        key_column: &str,
        keys: Vec<JsonValue>,
    ) -> ExtResult<u64> {
        let spec = self.resolve(table)?;
        let key_spec = spec
            .columns
            .iter()
            .find(|c| c.name == key_column)
            .ok_or_else(|| {
                ExtError::validation(format!(
                    "warehouse.delete: key_column {key_column:?} is not declared in \
                     contributes.warehouse_tables[{:?}].columns",
                    spec.name
                ))
            })?;
        let key_cast = placeholder_cast(&key_spec.ty);
        if keys.is_empty() {
            return Ok(0);
        }
        let full_table = full_table_name(self.extension_id, table);

        let mut sql = format!(
            "DELETE FROM {} WHERE {} IN (",
            quote_ident(&full_table),
            quote_ident(key_column)
        );
        for i in 0..keys.len() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("${}{key_cast}", i + 1));
        }
        sql.push_str(&format!(") AND \"tenant_id\" = ${}", keys.len() + 1));

        let mut args = PgArguments::default();
        for k in &keys {
            bind_column(&mut args, k, key_column)?;
        }
        args.add(self.tenant_id.to_owned())
            .map_err(|e| ExtError::extension_internal(format!("bind tenant_id: {e}")))?;

        sqlx::query_with::<Postgres, _>(&sql, args)
            .execute(self.metadata)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| ExtError::extension_internal(format!("warehouse.delete DELETE: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Binding helpers — ported from rubix `warehouse_write.rs`.
// ---------------------------------------------------------------------------

/// Build `($1, $2::cast, …), ($N, …)` for a multi-row INSERT, with explicit
/// per-column `::<type>` casts for types Postgres won't implicitly cast from
/// `text`/numeric (DATE, TIMESTAMPTZ, JSONB, UUID, …).
fn build_multi_row_placeholders(column_types: &[&str], rows: usize) -> String {
    let casts: Vec<&'static str> = column_types.iter().map(|t| placeholder_cast(t)).collect();
    let mut s = String::new();
    let mut idx: usize = 1;
    for r in 0..rows {
        if r > 0 {
            s.push_str(", ");
        }
        s.push('(');
        for (c, suffix) in casts.iter().enumerate() {
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

/// Map a Postgres column type to the `::<type>` placeholder suffix. Empty ⇒ no
/// cast (Postgres infers from the bound value or implicit-casts from text).
fn placeholder_cast(ty: &str) -> &'static str {
    let head = ty
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or(ty);
    match head.to_ascii_uppercase().as_str() {
        "DATE" => "::date",
        "TIMESTAMP" => {
            if ty.to_ascii_uppercase().contains("TIME ZONE") {
                "::timestamptz"
            } else {
                "::timestamp"
            }
        }
        "TIMESTAMPTZ" => "::timestamptz",
        "TIME" => "::time",
        "JSONB" => "::jsonb",
        "JSON" => "::json",
        "UUID" => "::uuid",
        _ => "",
    }
}

/// Bind one JSON value to `args`, dispatching on the value's shape. Postgres
/// implicit-casts most narrow types; the placeholder cast handles the rest.
fn bind_column(args: &mut PgArguments, value: &JsonValue, col_name: &str) -> ExtResult<()> {
    let map_bind = |e: sqlx::error::BoxDynError| {
        ExtError::extension_internal(format!("bind column {col_name:?}: {e}"))
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
        return Err(ExtError::validation(format!(
            "warehouse.write column {col_name:?}: unsigned value exceeds i64::MAX"
        )));
    }
    if let Some(f) = value.as_f64() {
        if f.is_nan() {
            return Err(ExtError::validation(format!(
                "warehouse.write column {col_name:?}: NaN cannot be bound"
            )));
        }
        args.add(f).map_err(map_bind)?;
        return Ok(());
    }
    if let Some(b) = value.as_bool() {
        args.add(b).map_err(map_bind)?;
        return Ok(());
    }
    // Arrays/objects bind as JSON(B).
    args.add(value.clone()).map_err(map_bind)?;
    Ok(())
}

/// The set of tables an extension may write, from its `warehouse_write` grant.
/// `None` if the extension declares no `warehouse_write` capability at all.
pub fn write_grant(manifest: &starter_ext_spi::Manifest) -> Option<BTreeSet<String>> {
    use starter_ext_spi::Capability;
    manifest.capabilities.iter().find_map(|c| match c {
        Capability::WarehouseWrite { tables } => Some(tables.iter().cloned().collect()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::manifest::{TableColumn as Col, WarehouseTableKind};

    fn ext() -> ExtensionId {
        ExtensionId::new("com.acme.devices").unwrap()
    }

    fn devices_spec() -> ContributeWarehouseTable {
        ContributeWarehouseTable {
            name: "devices".into(),
            columns: vec![
                Col { name: "device_id".into(), ty: "text".into(), default: None },
                Col { name: "barcode".into(), ty: "text".into(), default: None },
                Col { name: "location".into(), ty: "text".into(), default: Some("''".into()) },
            ],
            order_by: vec!["device_id".into()],
            engine: None,
            partition_by: None,
            ttl: None,
            kind: WarehouseTableKind::Table,
        }
    }

    #[test]
    fn sanitize_and_full_name() {
        assert_eq!(sanitize_extension_id(&ext()), "com_acme_devices");
        assert_eq!(full_table_name(&ext(), "devices"), "com_acme_devices__devices");
    }

    #[test]
    fn create_sql_prepends_tenant_and_builds_pk() {
        let sql = build_create_sql(&full_table_name(&ext(), "devices"), &devices_spec());
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS \"com_acme_devices__devices\""));
        assert!(sql.contains("\"tenant_id\" TEXT NOT NULL"));
        assert!(sql.contains("\"device_id\" text"));
        assert!(sql.contains("\"location\" text DEFAULT ''"));
        assert!(sql.contains("PRIMARY KEY (\"tenant_id\", \"device_id\")"));
    }

    #[test]
    fn validate_rejects_tenant_id_column() {
        let mut bad = devices_spec();
        bad.columns[0].name = "tenant_id".into();
        assert!(validate_table_entry(&bad).unwrap_err().contains("reserved"));
    }

    #[test]
    fn validate_rejects_order_by_outside_columns() {
        let mut bad = devices_spec();
        bad.order_by = vec!["nope".into()];
        assert!(validate_table_entry(&bad).unwrap_err().contains("order_by"));
    }

    #[test]
    fn placeholders_and_casts() {
        assert_eq!(
            build_multi_row_placeholders(&["TEXT", "timestamptz", "double precision"], 2),
            "($1, $2::timestamptz, $3), ($4, $5::timestamptz, $6)"
        );
    }

    #[test]
    fn write_grant_extracts_tables_from_caps() {
        use starter_ext_spi::Capability;
        // Exercise the grant-extraction logic directly over a capability list
        // (Manifest has no Default ctor; the helper only reads `.capabilities`).
        let caps = vec![
            Capability::WarehouseRead { tables: vec![] },
            Capability::WarehouseWrite {
                tables: vec!["devices".into()],
            },
        ];
        let g: BTreeSet<String> = caps
            .iter()
            .find_map(|c| match c {
                Capability::WarehouseWrite { tables } => Some(tables.iter().cloned().collect()),
                _ => None,
            })
            .unwrap();
        assert!(g.contains("devices"));
    }
}
