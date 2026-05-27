//! `rubix.warehouse.tables.list` — enumerate the hypertables the
//! warehouse exposes, with engine, retention, and approximate row
//! count.
//!
//! The TimescaleDB-flavoured shape preserves the ClickHouse DTO
//! field names (`engine`, `retention_days`, `row_count`) so the
//! frontend hooks don't need rewriting:
//!
//! - `engine`         — literal `"timescaledb_hypertable"` when the
//!                      table is a hypertable; `"postgres_table"`
//!                      for plain Postgres tables. The DTO field
//!                      is required, so we cannot leave it absent.
//! - `retention_days` — pulled from `timescaledb_information.jobs`
//!                      via [`starter_store_warehouse::retention::
//!                      snapshot_days`].
//! - `row_count`      — `approximate_row_count(name)` for
//!                      hypertables (cheap; uses chunk stats).

use async_trait::async_trait;
use rubix_spi::dto::warehouse::tables_list::{
    ClickhouseTableSummary, WarehouseTablesListRequest, WarehouseTablesListResponse,
};
use serde_json::Value;
use sqlx::Row;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::{retention, WarehouseClient};

pub struct WarehouseTablesListTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseTablesListTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseTablesListTool").finish()
    }
}

impl WarehouseTablesListTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for WarehouseTablesListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.tables.list".to_owned(),
            description: rubix_spi::dto::warehouse::tables_list::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _req: WarehouseTablesListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseTablesListRequest: {e}"),
            })?;

        // Enumerate every base table in `public` and classify it as
        // a Timescale hypertable or a plain Postgres table. Extension
        // tables (declared via `contributes.warehouse_tables[]`) are
        // plain Postgres tables, so a hypertables-only query would
        // hide them from the warehouse explorer.
        let rows = sqlx::query(
            "SELECT t.table_name, \
                    (h.hypertable_name IS NOT NULL) AS is_hypertable \
             FROM information_schema.tables AS t \
             LEFT JOIN timescaledb_information.hypertables AS h \
               ON h.hypertable_schema = t.table_schema \
              AND h.hypertable_name   = t.table_name \
             WHERE t.table_schema = 'public' \
               AND t.table_type   = 'BASE TABLE' \
             ORDER BY t.table_name ASC",
        )
        .fetch_all(self.client.pool())
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;

        let mut tables: Vec<ClickhouseTableSummary> = Vec::with_capacity(rows.len());
        for r in rows {
            let table_name: String = r.try_get("table_name").map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
            let is_hypertable: bool =
                r.try_get("is_hypertable").map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;

            let (engine, retention_days, row_count) = if is_hypertable {
                let retention_days = retention::snapshot_days(&self.client, &table_name)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|d| u32::try_from(d).ok());
                let row_count = approximate_row_count(&self.client, &table_name).await;
                ("timescaledb_hypertable".to_owned(), retention_days, row_count)
            } else {
                // Plain Postgres table — no retention policy, and
                // `approximate_row_count` is a hypertable-only
                // function. Use `pg_class.reltuples` (cheap, stale
                // but free) as the row-count proxy.
                let row_count = pg_reltuples_row_count(&self.client, &table_name).await;
                ("postgres_table".to_owned(), None, row_count)
            };

            tables.push(ClickhouseTableSummary {
                table_name,
                engine,
                retention_days,
                row_count,
            });
        }

        let count = tables.len();
        let summary = Diagnostic::new(
            MessageKey::parse("rubix.warehouse.tables.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = WarehouseTablesListResponse {
            summary,
            count,
            tables,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

/// `approximate_row_count` is the cheap chunk-stats probe documented
/// in the proposal. Failures (e.g. table not analyzed yet) collapse
/// to `None` rather than failing the list call.
async fn approximate_row_count(client: &WarehouseClient, table: &str) -> Option<u64> {
    let stmt = format!("SELECT approximate_row_count('{table}') AS n");
    let row = sqlx::query(&stmt)
        .fetch_optional(client.pool())
        .await
        .ok()??;
    let n: i64 = row.try_get("n").ok()?;
    u64::try_from(n).ok()
}

/// Cheap (planner-statistic) row count for plain Postgres tables.
/// `reltuples` is updated by VACUUM/ANALYZE and is `-1` until the
/// first ANALYZE; both stale and never-analyzed cases collapse to
/// `None` rather than reporting a misleading value.
async fn pg_reltuples_row_count(client: &WarehouseClient, table: &str) -> Option<u64> {
    let row = sqlx::query(
        "SELECT reltuples::bigint AS n FROM pg_class \
         WHERE oid = to_regclass($1)",
    )
    .bind(format!("public.{table}"))
    .fetch_optional(client.pool())
    .await
    .ok()??;
    let n: i64 = row.try_get("n").ok()?;
    u64::try_from(n).ok()
}
