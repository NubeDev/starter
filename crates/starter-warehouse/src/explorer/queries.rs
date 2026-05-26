// Forked from sql-studio (MIT) — https://github.com/frectonz/sql-studio
// Original copyright (c) frectonz. See NOTICES.md.
//
// Port of sql-studio's `mod clickhouse` query bodies (around
// `src/main.rs` line 3925 upstream). Rewritten to run through the
// existing `ChClient` from `starter-store-clickhouse` so the
// explorer reuses the warehouse's connection, auth, and W8
// `async_insert` discipline rather than opening a fresh client.
//
// Two intentional fidelity deltas vs. upstream:
//
//   * Row/column counts are deserialised as `i64`. ClickHouse's
//     `count()` returns `UInt64`; binding it to `i32` (as upstream
//     does) overflows the moment a table grows past ~2.1B rows.
//   * `table_data` is left stubbed to return `rows: []` exactly as
//     upstream does. PR 2 finishes it via a new
//     `ChClient::fetch_json` and is the only place in the explorer
//     that touches `starter-store-clickhouse`.
//
// Design notes: rubix/docs/design/warehouse/explorer/README.md.

use starter_store_clickhouse::{raw::JsonRows, ChClient, ChClientError};

use super::types::{
    Count, Erd, ErdColumn, ErdTable, Overview, Table, TableData, TableWithColumns, Tables,
    TablesWithColumns,
};

/// Hard-coded page size matching sql-studio's `ROWS_PER_PAGE`.
/// Kept here so PR 2 can read it without poking inside the upstream
/// fork.
pub const ROWS_PER_PAGE: i64 = 50;

/// Rows returned per `tables_with_columns` autocomplete entry.
/// Upstream truncates to 5 to keep the autocomplete payload
/// bounded; we keep the same cap.
const AUTOCOMPLETE_COL_LIMIT: usize = 5;

/// `(name, count)` for the column-count grouping. Renamed from the
/// upstream `ClickhouseCount` to keep the public types clean.
#[derive(serde::Deserialize, clickhouse::Row, Debug)]
struct NamedCount64 {
    pub name: String,
    pub count: i64,
}

/// Overview tiles: table / view / index counts, plus the per-table
/// row and column histograms.
pub async fn overview(ch: &ChClient, database: &str) -> Result<Overview, ChClientError> {
    let conn = ch.inner();

    // Exclude `Dictionary` engine entries: they're sourced from
    // external systems (e.g. Postgres) and a `count(*)` against them
    // forces a dictionary load. If the upstream source table is
    // missing the whole overview 500s. The explorer describes
    // tables, not dictionary sources, so this is also the right
    // semantics for the KPI tile.
    let tables: i64 = conn
        .query(
            "SELECT count(*) FROM system.tables \
             WHERE database = currentDatabase() AND engine != 'Dictionary'",
        )
        .fetch_one()
        .await?;

    let indexes: i64 = conn
        .query(
            "SELECT count(*) FROM system.columns \
             WHERE database = currentDatabase() \
             AND (is_in_primary_key = true OR is_in_sorting_key = true)",
        )
        .fetch_one()
        .await?;

    let views: i64 = conn
        .query(
            "SELECT count(*) FROM system.tables \
             WHERE database = currentDatabase() AND engine = 'View'",
        )
        .fetch_one()
        .await?;

    // Row counts: one `count(*)` per table. Same shape as upstream;
    // costly on a many-table database but acceptable for the
    // operator-facing overview. Skip `Dictionary` engines: a
    // `count(*)` against one forces ClickHouse to load the
    // dictionary, which roundtrips to its external source and
    // 500s the whole overview if that source is unavailable.
    let table_names: Vec<String> = conn
        .query(
            "SELECT name FROM system.tables \
             WHERE database = currentDatabase() AND engine != 'Dictionary'",
        )
        .fetch_all()
        .await?;

    let mut row_counts: Vec<Count> = Vec::with_capacity(table_names.len());
    for name in &table_names {
        let count: i64 = conn
            .query(&format!("SELECT count(*) FROM `{name}`"))
            .fetch_one()
            .await?;
        row_counts.push(Count {
            name: name.clone(),
            count,
        });
    }
    row_counts.sort_by(|a, b| b.count.cmp(&a.count));

    let mut column_counts: Vec<Count> = conn
        .query(
            "SELECT table AS name, count() AS count \
             FROM system.columns \
             WHERE database = currentDatabase() \
             GROUP BY table",
        )
        .fetch_all::<NamedCount64>()
        .await?
        .into_iter()
        .map(|c| Count {
            name: c.name,
            count: c.count,
        })
        .collect();
    column_counts.sort_by(|a, b| b.count.cmp(&a.count));

    let mut index_counts: Vec<Count> = Vec::with_capacity(table_names.len());
    for name in &table_names {
        let count: i64 = conn
            .query(
                "SELECT count(*) FROM system.columns \
                 WHERE database = currentDatabase() AND table = ? \
                 AND (is_in_primary_key = true OR is_in_sorting_key = true)",
            )
            .bind(name)
            .fetch_one()
            .await?;
        index_counts.push(Count {
            name: name.clone(),
            count,
        });
    }
    index_counts.sort_by(|a, b| b.count.cmp(&a.count));

    let size_on_disk = conn
        .query(
            "SELECT formatReadableSize(coalesce(sum(bytes), 0)) \
             FROM system.parts \
             WHERE database = currentDatabase() AND active",
        )
        .fetch_one::<String>()
        .await
        .unwrap_or_default();

    Ok(Overview {
        file_name: database.to_string(),
        size_on_disk,
        created: None,
        modified: None,
        tables: tables as i32,
        indexes: indexes as i32,
        triggers: 0,
        views: views as i32,
        row_counts,
        column_counts,
        index_counts,
    })
}

/// Tables list with per-table row counts.
pub async fn tables(ch: &ChClient) -> Result<Tables, ChClientError> {
    let conn = ch.inner();
    // Skip `Dictionary` engines for the same reason as `overview`:
    // `count(*)` against them would force a load against an
    // external source that may be missing.
    let names: Vec<String> = conn
        .query(
            "SELECT name FROM system.tables \
             WHERE database = currentDatabase() AND engine != 'Dictionary'",
        )
        .fetch_all()
        .await?;

    let mut out: Vec<Count> = Vec::with_capacity(names.len());
    for name in names {
        let count: i64 = conn
            .query(&format!("SELECT count(*) FROM `{name}`"))
            .fetch_one()
            .await?;
        out.push(Count { name, count });
    }
    // Upstream sorts ascending; preserve that.
    out.sort_by_key(|r| r.count);
    Ok(Tables { tables: out })
}

/// Single-table detail: CREATE statement, row count, on-disk size,
/// column / index counts.
pub async fn table(ch: &ChClient, name: &str) -> Result<Table, ChClientError> {
    let conn = ch.inner();

    let sql: Option<String> = conn
        .query(
            "SELECT create_table_query FROM system.tables \
             WHERE database = currentDatabase() AND name = ?",
        )
        .bind(name)
        .fetch_optional()
        .await?;

    let row_count: i64 = conn
        .query(&format!("SELECT count(*) FROM `{name}`"))
        .fetch_one()
        .await?;

    let table_size = conn
        .query(
            "SELECT formatReadableSize(coalesce(sum(bytes), 0)) \
             FROM system.parts \
             WHERE database = currentDatabase() AND table = ? AND active",
        )
        .bind(name)
        .fetch_one::<String>()
        .await
        .unwrap_or_default();

    let index_count: i64 = conn
        .query(
            "SELECT count(*) FROM system.columns \
             WHERE database = currentDatabase() AND table = ? \
             AND (is_in_primary_key = true OR is_in_sorting_key = true)",
        )
        .bind(name)
        .fetch_one()
        .await?;

    let column_count: i64 = conn
        .query(
            "SELECT count() FROM system.columns \
             WHERE database = currentDatabase() AND table = ?",
        )
        .bind(name)
        .fetch_one()
        .await?;

    Ok(Table {
        name: name.to_string(),
        sql,
        row_count,
        index_count: index_count as i32,
        column_count: column_count as i32,
        table_size,
    })
}

/// Just the column list for a table — backs `GET /tables/:name/columns`.
pub async fn columns(ch: &ChClient, name: &str) -> Result<Vec<String>, ChClientError> {
    let conn = ch.inner();
    let cols: Vec<String> = conn
        .query(
            "SELECT name FROM system.columns \
             WHERE database = currentDatabase() AND table = ? \
             ORDER BY position",
        )
        .bind(name)
        .fetch_all()
        .await?;
    Ok(cols)
}

/// `GET /tables/:name/data` — paged rows for a single table.
///
/// PR 2 finishes the upstream sql-studio stub: we read the column
/// list out of `system.columns` (so the response keeps the
/// upstream shape even for empty tables), then run a dynamic
/// `SELECT ... LIMIT ... OFFSET ...` through
/// [`ChClient::fetch_json`]. The query executes under
/// `SETTINGS readonly = 2` on the server (set by `fetch_json`),
/// and `fetch_json` itself refuses any write verb client-side.
pub async fn table_data(
    ch: &ChClient,
    name: &str,
    page: i64,
) -> Result<TableData, ChClientError> {
    let cols = columns(ch, name).await?;
    if cols.is_empty() {
        return Ok(TableData {
            columns: cols,
            rows: Vec::new(),
        });
    }
    let page = page.max(1);
    let offset = (page - 1) * ROWS_PER_PAGE;
    let select_list = cols
        .iter()
        .map(|c| format!("`{c}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let order_by = format!("`{}`", cols[0]);
    let sql = format!(
        "SELECT {select_list} FROM `{name}` ORDER BY {order_by} LIMIT {ROWS_PER_PAGE} OFFSET {offset}",
    );
    let JsonRows { rows, .. } = ch.fetch_json(&sql).await?;
    Ok(TableData {
        columns: cols,
        rows,
    })
}

/// `POST /query` runner. The handler is responsible for the
/// leading-token allow-list (`super::parse::classify`). This
/// function only forwards a pre-validated statement, relying on
/// `fetch_json`'s server-side `readonly=2` for defence in depth.
pub async fn query(ch: &ChClient, sql: &str) -> Result<JsonRows, ChClientError> {
    ch.fetch_json(sql).await
}

/// Autocomplete payload — every table with up to
/// [`AUTOCOMPLETE_COL_LIMIT`] column names each.
pub async fn tables_with_columns(ch: &ChClient) -> Result<TablesWithColumns, ChClientError> {
    let conn = ch.inner();
    let table_names: Vec<String> = conn
        .query(
            "SELECT name FROM system.tables \
             WHERE database = currentDatabase()",
        )
        .fetch_all()
        .await?;

    let mut tables = Vec::with_capacity(table_names.len());
    for table_name in table_names {
        let mut cols: Vec<String> = conn
            .query(
                "SELECT name FROM system.columns \
                 WHERE database = currentDatabase() AND table = ? \
                 ORDER BY position",
            )
            .bind(&table_name)
            .fetch_all()
            .await?;
        cols.truncate(AUTOCOMPLETE_COL_LIMIT);
        tables.push(TableWithColumns {
            table_name,
            columns: cols,
        });
    }
    Ok(TablesWithColumns { tables })
}

/// ERD payload. ClickHouse has no foreign keys, so `relationships`
/// is always empty — kept for UI shape parity with sql-studio.
pub async fn erd(ch: &ChClient) -> Result<Erd, ChClientError> {
    #[derive(clickhouse::Row, serde::Deserialize)]
    struct ColumnInfo {
        table: String,
        name: String,
        #[serde(rename = "type")]
        data_type: String,
        is_in_primary_key: u8,
    }

    let column_rows: Vec<ColumnInfo> = ch
        .inner()
        .query(
            "SELECT table, name, type, is_in_primary_key \
             FROM system.columns \
             WHERE database = currentDatabase() \
             ORDER BY table, position",
        )
        .fetch_all()
        .await?;

    let mut by_table: std::collections::BTreeMap<String, Vec<ErdColumn>> =
        std::collections::BTreeMap::new();
    for row in column_rows {
        by_table.entry(row.table).or_default().push(ErdColumn {
            name: row.name,
            data_type: row.data_type,
            nullable: false,
            is_primary_key: row.is_in_primary_key == 1,
        });
    }

    let tables = by_table
        .into_iter()
        .map(|(name, columns)| ErdTable { name, columns })
        .collect();

    Ok(Erd {
        tables,
        relationships: Vec::new(),
    })
}
