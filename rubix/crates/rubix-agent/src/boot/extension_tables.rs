//! Boot-time DDL for extension-owned warehouse tables.
//!
//! Walks the sealed [`ExtensionRegistry`] and for every Validated
//! extension's `contributes.warehouse_tables[]` entry, issues
//! `CREATE TABLE IF NOT EXISTS` + a companion `CREATE INDEX IF NOT
//! EXISTS` on the manifest's `order_by` columns. Idempotent: a
//! second boot is a no-op.
//!
//! Per-table failure logs at warn and continues — one bad manifest
//! cannot block the rest of the host from coming up. The summary
//! line at the end reports tables created vs. tables skipped so an
//! operator can spot drift in one log line.
//!
//! Postgres-only for now (the warehouse is Postgres/Timescale). A
//! ClickHouse follow-up would add an engine selector in front of
//! `build_create_sql`.

use std::sync::Arc;

use sqlx::Executor;
use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::manifest::{ContributeWarehouseTable, TableColumn};
use starter_ext_spi::{ExtensionId, LifecycleState};
use starter_store_warehouse::WarehouseClient;
use tracing::{info, warn};

use crate::extensions::warehouse_write::{full_table_name, sanitize_extension_id};

/// Outcome summary from one boot-DDL sweep.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExtensionTablesOutcome {
    /// Tables seen across every Validated extension.
    pub seen: usize,
    /// Tables for which `CREATE TABLE IF NOT EXISTS` succeeded.
    pub created_or_existing: usize,
    /// Tables skipped for reasons that aren't fatal (invalid name,
    /// `tenant_id` collision, DDL execution failed). One warn line
    /// per skip names the offending id.
    pub skipped: usize,
}

/// Walk the registry and create every declared extension table.
///
/// Returns the [`ExtensionTablesOutcome`] so the caller can log a
/// single summary line. Errors from individual statements are
/// logged and counted as `skipped`; this function only returns
/// `Err` if the iteration itself cannot proceed (currently never).
pub async fn create_extension_tables(
    registry: &Arc<ExtensionRegistry>,
    warehouse: &WarehouseClient,
) -> ExtensionTablesOutcome {
    let mut outcome = ExtensionTablesOutcome::default();
    for record in registry.iter_validated() {
        if record.state != LifecycleState::Validated {
            continue;
        }
        let Some(extension_id) = record.id.as_ref() else {
            continue;
        };
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        for entry in &manifest.contributes.warehouse_tables {
            outcome.seen += 1;
            match apply_one(warehouse, extension_id, entry).await {
                Ok(()) => outcome.created_or_existing += 1,
                Err(reason) => {
                    outcome.skipped += 1;
                    warn!(
                        target: "rubix.boot.extensions.tables",
                        extension = %extension_id.as_str(),
                        table = %entry.name,
                        reason = %reason,
                        "skipping extension-table DDL",
                    );
                }
            }
        }
    }
    info!(
        target: "rubix.boot.extensions.tables",
        seen = outcome.seen,
        created_or_existing = outcome.created_or_existing,
        skipped = outcome.skipped,
        "extension warehouse-table DDL applied",
    );
    outcome
}

/// Apply one entry — validation, CREATE TABLE, CREATE INDEX.
async fn apply_one(
    warehouse: &WarehouseClient,
    extension_id: &ExtensionId,
    entry: &ContributeWarehouseTable,
) -> Result<(), String> {
    validate_identifier(&entry.name, "table")?;
    for col in &entry.columns {
        validate_identifier(&col.name, "column")?;
        if col.name == "tenant_id" {
            return Err("column \"tenant_id\" is reserved (host stamps it on every insert)".into());
        }
    }
    for ob in &entry.order_by {
        // `order_by` may reference the host-prepended `tenant_id`,
        // so it's allowed even though it's not in `entry.columns`.
        if ob != "tenant_id" && !entry.columns.iter().any(|c| &c.name == ob) {
            return Err(format!(
                "order_by references column {ob:?} which is not in `columns`"
            ));
        }
    }

    let full_table = full_table_name(extension_id, &entry.name);
    let create_sql = build_create_sql(&full_table, entry);
    let pool = warehouse.pool();
    pool.execute(create_sql.as_str())
        .await
        .map_err(|e| format!("CREATE TABLE: {e}"))?;

    // Index on (tenant_id, order_by_cols) so the cleaner-style
    // workload (per-tenant time-window reads + ordered inserts)
    // gets a sensible default. Non-unique so extensions whose
    // natural keys aren't unique (ad-hoc append workloads) still
    // work without surprise PRIMARY KEY conflicts.
    if !entry.order_by.is_empty() {
        let idx_name = format!("{}__idx", sanitize_extension_id(extension_id));
        let idx_cols = std::iter::once("tenant_id")
            .chain(entry.order_by.iter().map(String::as_str))
            .map(quote_ident)
            .collect::<Vec<_>>()
            .join(", ");
        let index_sql = format!(
            "CREATE INDEX IF NOT EXISTS \"{}_{}\" ON \"{}\" ({})",
            idx_name, entry.name, full_table, idx_cols,
        );
        pool.execute(index_sql.as_str())
            .await
            .map_err(|e| format!("CREATE INDEX: {e}"))?;
    }
    Ok(())
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

fn quote_ident(s: &str) -> String {
    // Identifiers are pre-validated by `validate_identifier`, so we
    // just wrap them in double-quotes for parity with the
    // write-backend's quoting.
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Build the `CREATE TABLE IF NOT EXISTS` SQL.
fn build_create_sql(full_table: &str, entry: &ContributeWarehouseTable) -> String {
    let mut s = format!("CREATE TABLE IF NOT EXISTS \"{full_table}\" (\n");
    s.push_str("    \"tenant_id\" TEXT NOT NULL");
    for col in &entry.columns {
        s.push_str(",\n    ");
        s.push_str(&format_column(col));
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

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::manifest::TableColumn as Col;

    fn ext() -> ExtensionId {
        ExtensionId::new("com.acme.power").unwrap()
    }

    fn solar_spec() -> ContributeWarehouseTable {
        ContributeWarehouseTable {
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
                    default: Some("0".into()),
                },
            ],
            order_by: vec!["ts".into()],
            engine: None,
            partition_by: None,
            ttl: None,
        }
    }

    #[test]
    fn create_sql_includes_tenant_and_defaults() {
        let full = full_table_name(&ext(), "solar_panels");
        let sql = build_create_sql(&full, &solar_spec());
        assert!(sql.contains("\"tenant_id\" TEXT NOT NULL"));
        assert!(sql.contains("\"ts\" DOUBLE PRECISION"));
        assert!(sql.contains("\"kwh\" DOUBLE PRECISION DEFAULT 0"));
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS \"com_acme_power__solar_panels\""));
    }

    #[test]
    fn validate_identifier_rejects_bad_names() {
        assert!(validate_identifier("", "table").is_err());
        assert!(validate_identifier("1table", "table").is_err());
        assert!(validate_identifier("table-name", "table").is_err());
        assert!(validate_identifier("ok_name_123", "table").is_ok());
        assert!(validate_identifier("_ok", "table").is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn apply_one_refuses_tenant_id_column() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://test:test@127.0.0.1:1/test")
            .unwrap();
        let wh = WarehouseClient::from_pool(pool);
        let bad = ContributeWarehouseTable {
            name: "weather".into(),
            columns: vec![Col {
                name: "tenant_id".into(), // reserved
                ty: "TEXT".into(),
                default: None,
            }],
            order_by: vec![],
            engine: None,
            partition_by: None,
            ttl: None,
        };
        let err = apply_one(&wh, &ext(), &bad).await.expect_err("reserved");
        assert!(err.contains("reserved"), "got {err}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn apply_one_refuses_order_by_outside_columns() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://test:test@127.0.0.1:1/test")
            .unwrap();
        let wh = WarehouseClient::from_pool(pool);
        let bad = ContributeWarehouseTable {
            name: "weather".into(),
            columns: vec![Col {
                name: "ts".into(),
                ty: "DOUBLE PRECISION".into(),
                default: None,
            }],
            order_by: vec!["nope".into()],
            engine: None,
            partition_by: None,
            ttl: None,
        };
        let err = apply_one(&wh, &ext(), &bad)
            .await
            .expect_err("order_by mismatch");
        assert!(err.contains("order_by"), "got {err}");
    }
}
