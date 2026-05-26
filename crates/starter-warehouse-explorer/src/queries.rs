//! Postgres / TimescaleDB query implementations for the seven
//! explorer endpoints.
//!
//! Every query is a parameterised `sqlx::query*` call. The two
//! places that interpolate identifiers (`/tables/{name}` and
//! `/tables/{name}/data`) gate the value through
//! [`crate::validate::is_safe_identifier`] before reaching this
//! module — see the call-sites in `handlers.rs`.

use serde_json::Value as J;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::{Column, Row, TypeInfo};

use crate::types::{
    Autocomplete, AutocompleteTable, CountEntry, Erd, ErdColumn, ErdRelationship, ErdTable,
    Overview, Table, TableData, Tables,
};

/// Convert a single sqlx `PgRow` into a column-ordered
/// `Vec<serde_json::Value>` using best-effort type dispatch.
///
/// Unknown / unhandled column types fall back to a `<unsupported
/// type: …>` string rather than failing the whole row — the
/// explorer is a debugging UI and partial visibility beats a 500.
pub fn row_to_json(row: &PgRow) -> Vec<J> {
    row.columns()
        .iter()
        .enumerate()
        .map(|(i, col)| pg_value_to_json(row, i, col.type_info().name()))
        .collect()
}

fn pg_value_to_json(row: &PgRow, i: usize, ty: &str) -> J {
    use sqlx::ValueRef;
    // Null-check via the raw value ref — type-specific `try_get`
    // would map NULL to an error for non-Option targets.
    if let Ok(raw) = row.try_get_raw(i) {
        if raw.is_null() {
            return J::Null;
        }
    } else {
        return J::Null;
    }
    match ty {
        "BOOL" => row.try_get::<bool, _>(i).map(J::Bool).unwrap_or(J::Null),
        "INT2" => row
            .try_get::<i16, _>(i)
            .map(|v| J::Number(v.into()))
            .unwrap_or(J::Null),
        "INT4" => row
            .try_get::<i32, _>(i)
            .map(|v| J::Number(v.into()))
            .unwrap_or(J::Null),
        "INT8" => row
            .try_get::<i64, _>(i)
            .map(|v| J::Number(v.into()))
            .unwrap_or(J::Null),
        "FLOAT4" => row
            .try_get::<f32, _>(i)
            .ok()
            .and_then(|v| serde_json::Number::from_f64(v as f64).map(J::Number))
            .unwrap_or(J::Null),
        "FLOAT8" => row
            .try_get::<f64, _>(i)
            .ok()
            .and_then(|v| serde_json::Number::from_f64(v).map(J::Number))
            .unwrap_or(J::Null),
        "TEXT" | "VARCHAR" | "NAME" | "BPCHAR" | "CHAR" => row
            .try_get::<String, _>(i)
            .map(J::String)
            .unwrap_or(J::Null),
        "JSON" | "JSONB" => row.try_get::<J, _>(i).unwrap_or(J::Null),
        "UUID" => row
            .try_get::<sqlx::types::Uuid, _>(i)
            .map(|v| J::String(v.to_string()))
            .unwrap_or(J::Null),
        "TIMESTAMP" => row
            .try_get::<chrono::NaiveDateTime, _>(i)
            .map(|v| J::String(v.format("%Y-%m-%dT%H:%M:%S%.f").to_string()))
            .unwrap_or(J::Null),
        "TIMESTAMPTZ" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(i)
            .map(|v| J::String(v.to_rfc3339()))
            .unwrap_or(J::Null),
        "DATE" => row
            .try_get::<chrono::NaiveDate, _>(i)
            .map(|v| J::String(v.to_string()))
            .unwrap_or(J::Null),
        "TIME" => row
            .try_get::<chrono::NaiveTime, _>(i)
            .map(|v| J::String(v.to_string()))
            .unwrap_or(J::Null),
        "NUMERIC" => row
            .try_get::<sqlx::types::BigDecimal, _>(i)
            .map(|v| J::String(v.to_string()))
            .unwrap_or(J::Null),
        "BYTEA" => row
            .try_get::<Vec<u8>, _>(i)
            .map(|v| J::String(format!("\\x{}", hex_encode(&v))))
            .unwrap_or(J::Null),
        other => J::String(format!("<unsupported type: {other}>")),
    }
}

fn hex_encode(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

// ---------------------------------------------------------------- overview

/// Build the `/overview` payload in one round-trip per logical
/// section. The aggregate counts come from `pg_class`, the
/// per-table breakdowns from `pg_stat_user_tables` joined with
/// `information_schema.columns` / `pg_indexes`.
pub async fn overview(pool: &PgPool) -> Result<Overview, sqlx::Error> {
    // Database name + size.
    let (file_name, size_on_disk): (String, String) = sqlx::query_as(
        "SELECT current_database()::text, pg_size_pretty(pg_database_size(current_database()))",
    )
    .fetch_one(pool)
    .await?;

    // Aggregate counts. `r` = ordinary table, `i` = index, `v` =
    // view, `m` = materialized view. Triggers come from
    // `pg_trigger` minus the internal ones.
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind = 'r' AND n.nspname = 'public')::bigint,
            (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind = 'i' AND n.nspname = 'public')::bigint,
            (SELECT count(*) FROM pg_trigger WHERE NOT tgisinternal)::bigint,
            (SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relkind IN ('v', 'm') AND n.nspname = 'public')::bigint",
    )
    .fetch_one(pool)
    .await?;
    let (tables, indexes, triggers, views) = row;

    let row_counts: Vec<CountEntry> = sqlx::query_as::<_, (String, i64)>(
        "SELECT relname::text, n_live_tup::bigint
         FROM pg_stat_user_tables
         WHERE schemaname = 'public'
         ORDER BY relname",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, count)| CountEntry { name, count })
    .collect();

    let column_counts: Vec<CountEntry> = sqlx::query_as::<_, (String, i64)>(
        "SELECT table_name::text, count(*)::bigint
         FROM information_schema.columns
         WHERE table_schema = 'public'
         GROUP BY table_name
         ORDER BY table_name",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, count)| CountEntry { name, count })
    .collect();

    let index_counts: Vec<CountEntry> = sqlx::query_as::<_, (String, i64)>(
        "SELECT tablename::text, count(*)::bigint
         FROM pg_indexes
         WHERE schemaname = 'public'
         GROUP BY tablename
         ORDER BY tablename",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, count)| CountEntry { name, count })
    .collect();

    Ok(Overview {
        file_name,
        sqlite_version: None,
        size_on_disk,
        // Postgres does not record per-database create / modify
        // timestamps; the frontend reviver renders these as "—".
        created: None,
        modified: None,
        tables,
        indexes,
        triggers,
        views,
        row_counts,
        column_counts,
        index_counts,
    })
}

// ------------------------------------------------------------------ tables

pub async fn tables(pool: &PgPool) -> Result<Tables, sqlx::Error> {
    let entries: Vec<CountEntry> = sqlx::query_as::<_, (String, i64)>(
        "SELECT relname::text, n_live_tup::bigint
         FROM pg_stat_user_tables
         WHERE schemaname = 'public'
         ORDER BY relname",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(name, count)| CountEntry { name, count })
    .collect();
    Ok(Tables { tables: entries })
}

// ------------------------------------------------------------ table detail

/// Caller-validated identifier `name` is interpolated into the
/// SQL string for `pg_total_relation_size` (which takes a
/// `regclass` literal). Every other query in this function is
/// parameterised.
pub async fn table_detail(pool: &PgPool, name: &str) -> Result<Option<Table>, sqlx::Error> {
    // Existence check + sizes in one round-trip. `to_regclass`
    // returns `NULL` for an unknown name without raising.
    let exists: Option<(i64, String)> = sqlx::query_as(
        "SELECT
            COALESCE((SELECT n_live_tup FROM pg_stat_user_tables
                       WHERE schemaname = 'public' AND relname = $1), 0)::bigint,
            COALESCE(pg_size_pretty(pg_total_relation_size(
                (quote_ident('public') || '.' || quote_ident($1))::regclass)), '0 bytes')
         WHERE to_regclass(quote_ident('public') || '.' || quote_ident($1)) IS NOT NULL",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    let Some((row_count, table_size)) = exists else {
        return Ok(None);
    };

    let column_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = $1",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    let index_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM pg_indexes
         WHERE schemaname = 'public' AND tablename = $1",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    let sql = Some(synthesise_create_table(pool, name).await?);

    Ok(Some(Table {
        name: name.to_owned(),
        sql,
        row_count,
        index_count,
        column_count,
        table_size,
    }))
}

/// Hand-built `CREATE TABLE` synthesis from `pg_attribute` +
/// `pg_constraint`. Postgres has no built-in equivalent of
/// ClickHouse's `SHOW CREATE TABLE`; `pg_dump` is the closest and
/// is a separate binary we don't want to shell out to from a hot
/// HTTP path.
///
/// The output is informational — pretty-printed for the explorer
/// schema tab, not guaranteed to be round-trippable through
/// `psql`.
async fn synthesise_create_table(pool: &PgPool, name: &str) -> Result<String, sqlx::Error> {
    let cols: Vec<(String, String, bool, Option<String>)> = sqlx::query_as(
        "SELECT
            a.attname::text,
            pg_catalog.format_type(a.atttypid, a.atttypmod)::text,
            a.attnotnull,
            pg_get_expr(d.adbin, d.adrelid)
         FROM pg_attribute a
         LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
         WHERE a.attrelid = (quote_ident('public') || '.' || quote_ident($1))::regclass
           AND a.attnum > 0
           AND NOT a.attisdropped
         ORDER BY a.attnum",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;

    let constraints: Vec<(String, String)> = sqlx::query_as(
        "SELECT conname::text, pg_get_constraintdef(oid)::text
         FROM pg_constraint
         WHERE conrelid = (quote_ident('public') || '.' || quote_ident($1))::regclass
         ORDER BY conname",
    )
    .bind(name)
    .fetch_all(pool)
    .await?;

    let mut out = format!("CREATE TABLE public.{name} (\n");
    let total = cols.len();
    for (i, (col, ty, notnull, default)) in cols.into_iter().enumerate() {
        out.push_str(&format!("    {col} {ty}"));
        if notnull {
            out.push_str(" NOT NULL");
        }
        if let Some(d) = default {
            out.push_str(&format!(" DEFAULT {d}"));
        }
        if i + 1 < total || !constraints.is_empty() {
            out.push(',');
        }
        out.push('\n');
    }
    let ct = constraints.len();
    for (i, (cname, cdef)) in constraints.into_iter().enumerate() {
        out.push_str(&format!("    CONSTRAINT {cname} {cdef}"));
        if i + 1 < ct {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(");\n");
    Ok(out)
}

// -------------------------------------------------------------- table data

/// Caller-validated identifier `name` is interpolated into the
/// `SELECT * FROM …` statement after passing
/// [`crate::validate::is_safe_identifier`].
pub async fn table_data(
    pool: &PgPool,
    name: &str,
    page: i64,
) -> Result<TableData, sqlx::Error> {
    let limit = crate::PAGE_SIZE;
    let offset = limit * (page.max(1) - 1);

    // Read in a tight read-only transaction. Even though
    // `SELECT *` is harmless, this matches the `/query` posture
    // and means a typo in the validator could never mutate.
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;

    let sql = format!("SELECT * FROM public.{name} LIMIT {limit} OFFSET {offset}");
    let rows = sqlx::query(&sql).fetch_all(&mut *tx).await?;
    tx.commit().await?;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_owned()).collect())
        .unwrap_or_else(|| Vec::new());
    let rows_json: Vec<Vec<J>> = rows.iter().map(row_to_json).collect();

    // Fall back to a metadata-only query when no rows came back
    // so the column header still renders.
    let columns = if columns.is_empty() {
        sqlx::query_scalar::<_, String>(
            "SELECT column_name::text FROM information_schema.columns
             WHERE table_schema = 'public' AND table_name = $1
             ORDER BY ordinal_position",
        )
        .bind(name)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        columns
    };

    Ok(TableData {
        columns,
        rows: rows_json,
    })
}

// ------------------------------------------------------------------ /query

/// Execute arbitrary user SQL inside a `READ ONLY DEFERRABLE`
/// transaction with a hard `statement_timeout`. The engine
/// rejects mutations with SQLSTATE `25006`, which `sqlx` surfaces
/// as `sqlx::Error::Database`. We do **no** string-level SQL
/// parsing — the engine is the only safe gate.
pub async fn run_query(pool: &PgPool, sql: &str) -> Result<TableData, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY DEFERRABLE")
        .execute(&mut *tx)
        .await?;
    let timeout = crate::QUERY_STATEMENT_TIMEOUT_MS;
    // `SET LOCAL` scopes to the current transaction; rollback
    // restores the session default.
    sqlx::query(&format!("SET LOCAL statement_timeout = {timeout}"))
        .execute(&mut *tx)
        .await?;

    let rows = sqlx::query(sql).fetch_all(&mut *tx).await?;
    tx.commit().await?;

    let columns: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_owned()).collect())
        .unwrap_or_default();
    let rows_json: Vec<Vec<J>> = rows.iter().map(row_to_json).collect();

    Ok(TableData {
        columns,
        rows: rows_json,
    })
}

// ----------------------------------------------------------- /autocomplete

pub async fn autocomplete(pool: &PgPool) -> Result<Autocomplete, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text
         FROM information_schema.columns
         WHERE table_schema = 'public'
         ORDER BY table_name, ordinal_position",
    )
    .fetch_all(pool)
    .await?;

    let mut tables: Vec<AutocompleteTable> = Vec::new();
    for (table, col) in rows {
        match tables.last_mut() {
            Some(last) if last.table_name == table => last.columns.push(col),
            _ => tables.push(AutocompleteTable {
                table_name: table,
                columns: vec![col],
            }),
        }
    }
    Ok(Autocomplete { tables })
}

// -------------------------------------------------------------------- /erd

pub async fn erd(pool: &PgPool) -> Result<Erd, sqlx::Error> {
    // Tables + columns. PK membership comes from
    // `pg_constraint(contype='p')` projected through
    // `unnest(conkey)`.
    let col_rows: Vec<(String, String, String, bool, bool)> = sqlx::query_as(
        "WITH pks AS (
            SELECT
                cl.relname::text  AS tbl,
                a.attname::text   AS col
            FROM pg_constraint c
            JOIN pg_namespace  n  ON n.oid  = c.connamespace AND n.nspname = 'public'
            JOIN pg_class      cl ON cl.oid = c.conrelid
            JOIN unnest(c.conkey) WITH ORDINALITY AS k(attnum, ord) ON TRUE
            JOIN pg_attribute  a  ON a.attrelid = c.conrelid AND a.attnum = k.attnum
            WHERE c.contype = 'p'
         )
         SELECT
            c.table_name::text,
            c.column_name::text,
            c.data_type::text,
            (c.is_nullable = 'YES'),
            EXISTS (SELECT 1 FROM pks p
                    WHERE p.tbl = c.table_name AND p.col = c.column_name)
         FROM information_schema.columns c
         WHERE c.table_schema = 'public'
         ORDER BY c.table_name, c.ordinal_position",
    )
    .fetch_all(pool)
    .await?;

    let mut tables: Vec<ErdTable> = Vec::new();
    for (table, col, ty, nullable, pk) in col_rows {
        let column = ErdColumn {
            name: col,
            data_type: ty,
            nullable,
            is_primary_key: pk,
        };
        match tables.last_mut() {
            Some(last) if last.name == table => last.columns.push(column),
            _ => tables.push(ErdTable {
                name: table,
                columns: vec![column],
            }),
        }
    }

    let rel_rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT
            cl1.relname::text                                AS from_table,
            a1.attname::text                                 AS from_column,
            cl2.relname::text                                AS to_table,
            a2.attname::text                                 AS to_column
         FROM pg_constraint c
         JOIN pg_namespace  n   ON n.oid  = c.connamespace AND n.nspname = 'public'
         JOIN pg_class      cl1 ON cl1.oid = c.conrelid
         JOIN pg_class      cl2 ON cl2.oid = c.confrelid
         JOIN unnest(c.conkey)  WITH ORDINALITY AS k1(attnum, ord) ON TRUE
         JOIN unnest(c.confkey) WITH ORDINALITY AS k2(attnum, ord) ON k2.ord = k1.ord
         JOIN pg_attribute  a1 ON a1.attrelid = c.conrelid  AND a1.attnum = k1.attnum
         JOIN pg_attribute  a2 ON a2.attrelid = c.confrelid AND a2.attnum = k2.attnum
         WHERE c.contype = 'f'
         ORDER BY cl1.relname, a1.attname",
    )
    .fetch_all(pool)
    .await?;

    let relationships = rel_rows
        .into_iter()
        .map(|(ft, fc, tt, tc)| ErdRelationship {
            from_table: ft,
            from_column: fc,
            to_table: tt,
            to_column: tc,
        })
        .collect();

    Ok(Erd {
        tables,
        relationships,
    })
}
