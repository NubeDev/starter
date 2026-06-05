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
    /// Entries the host intentionally did not create because their
    /// `kind` opts out of host-managed DDL (e.g. continuous
    /// aggregates owned by the extension's post-load script).
    /// Counted separately from `skipped` because it's a routine
    /// outcome, not a failure.
    pub deferred_to_extension: usize,
}

/// Walk the registry and create every declared extension table.
///
/// Tables are created **synchronously** — the warehouse write/read
/// path needs the schema to exist before the host starts serving. The
/// companion `order_by` indexes are *not* built here: they are a
/// query-time optimisation, and on an adopted table holding hundreds
/// of millions of rows a plain `CREATE INDEX` can run for a very long
/// time. Building them inline blocked boot (the HTTP listener only
/// opens after this returns), so an operator pointing a fresh host at
/// an existing TimescaleDB saw boot hang in `CREATE INDEX` — and
/// `CONCURRENTLY` is no escape hatch, because TimescaleDB rejects it
/// on hypertables. Instead the index statements are collected and
/// handed to [`spawn_index_build`], which builds them on a background
/// task after this returns; the listener comes up immediately and the
/// indexes materialise behind it.
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
    let mut pending_indexes: Vec<String> = Vec::new();
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
        let one = create_tables_collect_indexes(
            warehouse,
            extension_id,
            manifest,
            &mut pending_indexes,
        )
        .await;
        outcome.seen += one.seen;
        outcome.created_or_existing += one.created_or_existing;
        outcome.skipped += one.skipped;
        outcome.deferred_to_extension += one.deferred_to_extension;
    }
    info!(
        target: "rubix.boot.extensions.tables",
        seen = outcome.seen,
        created_or_existing = outcome.created_or_existing,
        skipped = outcome.skipped,
        deferred_to_extension = outcome.deferred_to_extension,
        pending_indexes = pending_indexes.len(),
        "extension warehouse-table DDL applied (indexes build in background)",
    );
    spawn_index_build(warehouse.pool().clone(), pending_indexes);
    outcome
}

/// Build the collected `order_by` indexes on a detached task so boot
/// never blocks on them.
///
/// Each statement is `CREATE INDEX IF NOT EXISTS`, so it is a no-op
/// once the index exists — a second boot does no work. Failures log at
/// warn and the next index is still attempted; a slow or failed index
/// degrades query plans but must not wedge the host.
///
/// For TimescaleDB hypertables the statement carries
/// `WITH (timescaledb.transaction_per_chunk)` so the build commits
/// chunk-by-chunk instead of holding one lock across the whole
/// hypertable for its entire (potentially very long) duration — the
/// hypertable-native stand-in for `CONCURRENTLY`, which TimescaleDB
/// refuses on hypertables.
///
/// # Single-flight
///
/// The build is guarded by a **session-level Postgres advisory lock**
/// ([`INDEX_BUILD_ADVISORY_LOCK_KEY`]) taken with the non-blocking
/// `pg_try_advisory_lock`. The index statements are idempotent
/// (`IF NOT EXISTS`), so concurrency could never produce a *wrong*
/// result — but on the adopted 955M-row hypertable a `CREATE INDEX`
/// runs for hours, and without this lock every agent restart (or a
/// watchdog bounce, or a second instance) span *another* concurrent
/// `CREATE INDEX` that then serialised behind the first on the
/// table's `ShareLock`, piling up redundant multi-hour backends on the
/// shared production DB. The lock makes that a no-op: a second builder
/// that can't grab the lock logs and exits, leaving the one in-flight
/// build to finish.
///
/// The lock is **session-scoped on a dedicated connection held for the
/// whole build**, not transaction-scoped — so it spans the several
/// `CREATE INDEX` statements, and (crucially) is released
/// automatically when this process dies and the connection closes. A
/// killed agent therefore never leaves the lock stuck: the next boot
/// reacquires it cleanly.
fn spawn_index_build(pool: sqlx::PgPool, statements: Vec<String>) {
    if statements.is_empty() {
        return;
    }
    tokio::spawn(async move {
        // Hold one dedicated connection for the lifetime of the build
        // so the session advisory lock persists across every
        // statement and releases on connection close (incl. crash).
        let mut guard = match pool.acquire().await {
            Ok(conn) => conn,
            Err(e) => {
                warn!(
                    target: "rubix.boot.extensions.tables",
                    error = %e,
                    "could not acquire connection for background index build; skipping",
                );
                return;
            }
        };

        let got_lock = sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_lock($1)")
            .bind(INDEX_BUILD_ADVISORY_LOCK_KEY)
            .fetch_one(guard.as_mut())
            .await
            .unwrap_or(false);
        if !got_lock {
            info!(
                target: "rubix.boot.extensions.tables",
                "another instance is already building extension indexes \
                 (advisory lock held); skipping background build",
            );
            return;
        }

        let total = statements.len();
        for (i, base_sql) in statements.into_iter().enumerate() {
            // The advisory lock stops *this host* from spawning a
            // duplicate, but it releases when a crashed/killed agent's
            // connection dies — while the `CREATE INDEX` it launched
            // keeps running *server-side* as an orphan. Issuing another
            // one would then block behind that orphan's ShareLock and
            // rebuild the pile-up. So also skip any index already being
            // built on the server, regardless of who launched it.
            if let Some(name) = index_name_of(&base_sql) {
                if index_build_in_progress(&pool, name).await {
                    info!(
                        target: "rubix.boot.extensions.tables",
                        index = i + 1,
                        total,
                        index_name = name,
                        "extension index already building server-side \
                         (orphan from a prior agent?); skipping",
                    );
                    continue;
                }
            }
            let sql = with_hypertable_index_options(&pool, &base_sql).await;
            info!(
                target: "rubix.boot.extensions.tables",
                index = i + 1,
                total,
                "building extension index (background)",
            );
            if let Err(e) = pool.execute(sql.as_str()).await {
                warn!(
                    target: "rubix.boot.extensions.tables",
                    error = %e,
                    sql = %sql,
                    "background extension-index build failed",
                );
            }
        }
        info!(
            target: "rubix.boot.extensions.tables",
            total,
            "background extension-index build complete",
        );
        // Lock also releases on connection close; unlock explicitly so
        // a long-lived pooled connection frees it promptly.
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(INDEX_BUILD_ADVISORY_LOCK_KEY)
            .execute(guard.as_mut())
            .await;
    });
}

/// Advisory-lock key for the background extension-index build (see
/// [`spawn_index_build`]). Arbitrary fixed constant — `pg_advisory_*`
/// locks share a single `bigint` keyspace, so the only requirement is
/// that it not collide with another advisory lock the host takes.
/// (`0x52 0x42 0x58 0x49 0x44 0x58` — ASCII `RBXIDX`.)
const INDEX_BUILD_ADVISORY_LOCK_KEY: i64 = 0x5242_5849_4458;

/// Extract the index name from a `CREATE INDEX IF NOT EXISTS "<name>"
/// …` statement (the quoted token after the `EXISTS`), or `None` if the
/// shape doesn't match. Used to look the index up in `pg_stat_activity`.
fn index_name_of(create_sql: &str) -> Option<&str> {
    create_sql
        .split_once("EXISTS \"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(name, _)| name)
}

/// Is a `CREATE INDEX` for `index_name` already running on the server
/// (any backend, including an orphan left by a killed agent)? Best
/// effort: a probe error returns `false`, so we fall through to the
/// idempotent `CREATE INDEX IF NOT EXISTS` rather than skip wrongly.
///
/// Matches on the index name appearing in an active `CREATE INDEX`
/// statement; the name is host-generated (`<ext>__idx_<table>`) and
/// validated, so it can't carry a SQL-injection / wildcard payload.
async fn index_build_in_progress(pool: &sqlx::PgPool, index_name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM pg_stat_activity \
         WHERE state = 'active' \
           AND pid <> pg_backend_pid() \
           AND query ILIKE '%CREATE INDEX%' || $1 || '%'",
    )
    .bind(index_name)
    .fetch_one(pool)
    .await
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// If the `ON "<table>"` of `base_sql` names a TimescaleDB hypertable,
/// append `WITH (timescaledb.transaction_per_chunk)`. Best-effort: any
/// probe error (e.g. TimescaleDB not installed) leaves the statement
/// unchanged, which is correct for a plain Postgres table.
async fn with_hypertable_index_options(pool: &sqlx::PgPool, base_sql: &str) -> String {
    let Some(table) = base_sql
        .split_once(" ON \"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(name, _)| name)
    else {
        return base_sql.to_owned();
    };
    let is_hypertable = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM timescaledb_information.hypertables \
         WHERE hypertable_name = $1",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .map(|n| n > 0)
    .unwrap_or(false);
    if is_hypertable {
        format!("{base_sql} WITH (timescaledb.transaction_per_chunk)")
    } else {
        base_sql.to_owned()
    }
}

/// Create every host-managed table declared by a single manifest,
/// deferring the companion indexes to the background single-flight
/// builder.
///
/// Used at install-time by
/// [`crate::extensions::ExtensionTablesInstallHook`]. For a normal
/// fresh install the tables are brand-new and empty, so the index
/// build is instant either way — but installing a bundle that
/// **adopts** an existing table (the `com.nubeio.rubixos` Timescale
/// case, where `histories` already holds ~955M rows) must not block
/// the install HTTP request on a multi-hour `CREATE INDEX`. So this
/// mirrors boot ([`create_extension_tables`]): `CREATE TABLE IF NOT
/// EXISTS` runs inline (metadata-only, cheap) and the `order_by`
/// indexes are handed to [`spawn_index_build`], which guards them with
/// the same advisory lock — so an install during a boot-time build (or
/// vice-versa) can't stack a second concurrent `CREATE INDEX` on the
/// hypertable. Idempotent (`CREATE TABLE / INDEX IF NOT EXISTS`); the
/// returned outcome counts what this one manifest contributed.
pub async fn create_tables_for_manifest(
    warehouse: &WarehouseClient,
    extension_id: &ExtensionId,
    manifest: &starter_ext_spi::Manifest,
) -> ExtensionTablesOutcome {
    let mut indexes = Vec::new();
    let outcome =
        create_tables_collect_indexes(warehouse, extension_id, manifest, &mut indexes).await;
    spawn_index_build(warehouse.pool().clone(), indexes);
    outcome
}

/// Create the tables for one manifest and push each table's `order_by`
/// index statement into `indexes` for the caller to run (inline or
/// backgrounded). Shared core of [`create_tables_for_manifest`] and
/// [`create_extension_tables`].
async fn create_tables_collect_indexes(
    warehouse: &WarehouseClient,
    extension_id: &ExtensionId,
    manifest: &starter_ext_spi::Manifest,
    indexes: &mut Vec<String>,
) -> ExtensionTablesOutcome {
    let mut outcome = ExtensionTablesOutcome::default();
    for entry in &manifest.contributes.warehouse_tables {
        outcome.seen += 1;
        if !entry.kind.host_manages_ddl() {
            // The entry exists in the registry so the per-call
            // allowlist gate still authorises templates that
            // reference it, but creation is the extension's
            // responsibility (e.g. a CAGG installed by
            // `scripts/post-load.sql`). Emitting a plain table
            // here would race the materialised view and leave
            // the relation as an empty stub.
            outcome.deferred_to_extension += 1;
            info!(
                target: "rubix.boot.extensions.tables",
                extension = %extension_id.as_str(),
                table = %entry.name,
                kind = ?entry.kind,
                "deferring DDL to extension (non-table kind)",
            );
            continue;
        }
        match apply_table(warehouse, extension_id, entry).await {
            Ok(index_sql) => {
                outcome.created_or_existing += 1;
                if let Some(sql) = index_sql {
                    indexes.push(sql);
                }
            }
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
    outcome
}

/// Apply one entry — validation + `CREATE TABLE` — and return the
/// companion `CREATE INDEX` statement for the caller to run (`None`
/// when the entry declares no `order_by`).
async fn apply_table(
    warehouse: &WarehouseClient,
    extension_id: &ExtensionId,
    entry: &ContributeWarehouseTable,
) -> Result<Option<String>, String> {
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
    warehouse
        .pool()
        .execute(create_sql.as_str())
        .await
        .map_err(|e| format!("CREATE TABLE: {e}"))?;

    Ok(build_index_sql(extension_id, &full_table, entry))
}

/// The `order_by` index statement for one entry, or `None` if the
/// entry declares no `order_by`.
///
/// Index on `(tenant_id, order_by_cols)` so the cleaner-style workload
/// (per-tenant time-window reads + ordered inserts) gets a sensible
/// default. Non-unique so extensions whose natural keys aren't unique
/// (ad-hoc append workloads) still work without surprise PRIMARY KEY
/// conflicts.
fn build_index_sql(
    extension_id: &ExtensionId,
    full_table: &str,
    entry: &ContributeWarehouseTable,
) -> Option<String> {
    if entry.order_by.is_empty() {
        return None;
    }
    let idx_name = format!("{}__idx", sanitize_extension_id(extension_id));
    let idx_cols = std::iter::once("tenant_id")
        .chain(entry.order_by.iter().map(String::as_str))
        .map(quote_ident)
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "CREATE INDEX IF NOT EXISTS \"{}_{}\" ON \"{}\" ({})",
        idx_name, entry.name, full_table, idx_cols,
    ))
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
    use starter_ext_spi::manifest::{TableColumn as Col, WarehouseTableKind};

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
            kind: WarehouseTableKind::Table,
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
    fn build_index_sql_prefixes_tenant_and_returns_none_without_order_by() {
        let full = full_table_name(&ext(), "solar_panels");
        let sql = build_index_sql(&ext(), &full, &solar_spec()).expect("has order_by");
        assert_eq!(
            sql,
            "CREATE INDEX IF NOT EXISTS \"com_acme_power__idx_solar_panels\" \
             ON \"com_acme_power__solar_panels\" (\"tenant_id\", \"ts\")"
        );

        let mut no_order = solar_spec();
        no_order.order_by.clear();
        assert!(build_index_sql(&ext(), &full, &no_order).is_none());
    }

    #[test]
    fn index_name_of_extracts_quoted_name() {
        let full = full_table_name(&ext(), "solar_panels");
        let sql = build_index_sql(&ext(), &full, &solar_spec()).expect("has order_by");
        assert_eq!(
            index_name_of(&sql),
            Some("com_acme_power__idx_solar_panels")
        );
        assert_eq!(index_name_of("CREATE TABLE foo (x int)"), None);
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
            kind: WarehouseTableKind::Table,
        };
        let err = apply_table(&wh, &ext(), &bad).await.expect_err("reserved");
        assert!(err.contains("reserved"), "got {err}");
    }

    #[test]
    fn continuous_aggregate_kind_defers_ddl_to_extension() {
        // The lone reliable signal that the boot step will skip the
        // entry: `host_manages_ddl()` returns `false`. The outer
        // loop in `create_extension_tables` short-circuits on that
        // before reaching `apply_one`, which is what keeps the host
        // from racing the extension's `post-load.sql` and stamping
        // a plain table over a continuous aggregate.
        assert!(WarehouseTableKind::Table.host_manages_ddl());
        assert!(!WarehouseTableKind::ContinuousAggregate.host_manages_ddl());

        // Sanity check: an entry flagged as a CAGG round-trips with
        // its kind preserved (so a `deny_unknown_fields` parse of
        // `block.yaml` actually surfaces the flag to the host).
        let cagg = ContributeWarehouseTable {
            name: "histories_1m".into(),
            columns: vec![Col {
                name: "bucket".into(),
                ty: "TIMESTAMPTZ".into(),
                default: None,
            }],
            order_by: vec!["bucket".into()],
            engine: None,
            partition_by: None,
            ttl: None,
            kind: WarehouseTableKind::ContinuousAggregate,
        };
        assert!(!cagg.kind.host_manages_ddl());
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
            kind: WarehouseTableKind::Table,
        };
        let err = apply_table(&wh, &ext(), &bad)
            .await
            .expect_err("order_by mismatch");
        assert!(err.contains("order_by"), "got {err}");
    }
}
