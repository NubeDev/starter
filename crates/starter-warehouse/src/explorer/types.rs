// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Response DTOs for the ClickHouse explorer sub-router. Ported
// verbatim from sql-studio's `mod responses` (around line 5150 of
// the upstream `src/main.rs`) with these intentional deltas:
//
//   * `Overview::sqlite_version` is dropped (CH-only fork).
//   * `Overview::db_size` is renamed to `size_on_disk` to reflect
//     what we actually compute from `system.parts`.
//   * `Metadata` is dropped — server lifecycle lives in
//     `starter-server`, not here.
//   * Doc design lives at
//     `rubix/docs/design/warehouse/explorer/README.md` (per
//     HOW-TO-CODE.md §0a).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Dashboard overview payload. The UI's index route renders the
/// large counters and the row/column histograms straight off this.
#[derive(Serialize)]
pub struct Overview {
    pub file_name: String,
    /// Human-readable size string from `formatReadableSize(sum(bytes))`
    /// over `system.parts`. Empty when the engine doesn't report
    /// parts (e.g. pure-view databases).
    pub size_on_disk: String,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub tables: i32,
    pub indexes: i32,
    pub triggers: i32,
    pub views: i32,
    pub row_counts: Vec<Count>,
    pub column_counts: Vec<Count>,
    pub index_counts: Vec<Count>,
}

/// `(name, count)` pair used in the overview histograms and the
/// tables-list payload. The `clickhouse::Row` derive lets us pull
/// the column-count grouping straight out of `system.columns`
/// without an intermediate type.
#[derive(Serialize, Deserialize, clickhouse::Row, Debug, Clone)]
pub struct Count {
    pub name: String,
    pub count: i64,
}

/// Top-level tables list response.
#[derive(Serialize)]
pub struct Tables {
    pub tables: Vec<Count>,
}

/// Per-table detail: DDL, size, row/column/index counts.
#[derive(Serialize)]
pub struct Table {
    pub name: String,
    pub sql: Option<String>,
    pub row_count: i64,
    pub index_count: i32,
    pub column_count: i32,
    pub table_size: String,
}

/// Paged rows for a table. PR 1 returns `rows: []` and lets PR 2
/// finish the stub via `ChClient::fetch_json`. The columns vector
/// is populated either way so the UI can render headers.
#[derive(Serialize)]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Autocomplete payload — every table with up to N column names.
#[derive(Serialize)]
pub struct TablesWithColumns {
    pub tables: Vec<TableWithColumns>,
}

#[derive(Serialize)]
pub struct TableWithColumns {
    pub table_name: String,
    pub columns: Vec<String>,
}

/// ERD payload. ClickHouse has no foreign keys, so
/// `relationships` is always empty — kept in the shape for UI
/// parity with sql-studio.
#[derive(Serialize)]
pub struct Erd {
    pub tables: Vec<ErdTable>,
    pub relationships: Vec<ErdRelationship>,
}

#[derive(Serialize)]
pub struct ErdTable {
    pub name: String,
    pub columns: Vec<ErdColumn>,
}

#[derive(Serialize)]
pub struct ErdColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
}

#[derive(Serialize)]
pub struct ErdRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}
