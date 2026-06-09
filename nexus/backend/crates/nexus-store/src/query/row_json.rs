//! Decode a Postgres row into a JSON object keyed by column name.
//!
//! sqlx hands back dynamically-typed `PgRow`s; the query path is schemaless (the
//! user's SQL decides the columns), so each value is decoded by inspecting the
//! column's Postgres type name and trying the matching Rust type. Types with no
//! direct mapping fall back to their text representation rather than failing the
//! whole query — a panel can still render an unfamiliar column as a string.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use nexus_spi::dto::query::{ColumnSchema, ResultColumnType};
use serde_json::{Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{Column, Row, TypeInfo};

/// Derive the column schema (name + coarse type) from a row's columns.
pub fn columns_of(row: &PgRow) -> Vec<ColumnSchema> {
    row.columns()
        .iter()
        .map(|c| ColumnSchema {
            name: c.name().to_string(),
            column_type: coarse_type(c.type_info().name()),
        })
        .collect()
}

/// Convert one row to a JSON object. Decoding errors for a single cell surface
/// as JSON `null` rather than aborting the query.
pub fn row_to_object(row: &PgRow) -> Value {
    let mut obj = Map::with_capacity(row.columns().len());
    for col in row.columns() {
        let name = col.name();
        obj.insert(
            name.to_string(),
            decode_cell(row, col.ordinal(), col.type_info().name()),
        );
    }
    Value::Object(obj)
}

/// Coarse type for the frontend, from the Postgres type name.
fn coarse_type(pg_type: &str) -> ResultColumnType {
    match pg_type {
        "BOOL" => ResultColumnType::Bool,
        "INT2" | "INT4" | "INT8" => ResultColumnType::Int,
        "FLOAT4" | "FLOAT8" | "NUMERIC" => ResultColumnType::Float,
        "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" | "UUID" => ResultColumnType::String,
        "TIMESTAMP" | "TIMESTAMPTZ" | "DATE" | "TIME" => ResultColumnType::Timestamp,
        _ => ResultColumnType::Other,
    }
}

/// Decode a single cell by Postgres type name. The `try_get` calls handle NULL
/// (an absent value becomes JSON `null`).
fn decode_cell(row: &PgRow, idx: usize, pg_type: &str) -> Value {
    match pg_type {
        "BOOL" => json_opt(row.try_get::<Option<bool>, _>(idx)),
        "INT2" => json_opt(row.try_get::<Option<i16>, _>(idx).map(|o| o.map(i64::from))),
        "INT4" => json_opt(row.try_get::<Option<i32>, _>(idx).map(|o| o.map(i64::from))),
        "INT8" => json_opt(row.try_get::<Option<i64>, _>(idx)),
        "FLOAT4" => json_opt(row.try_get::<Option<f32>, _>(idx).map(|o| o.map(f64::from))),
        "FLOAT8" => json_opt(row.try_get::<Option<f64>, _>(idx)),
        "TEXT" | "VARCHAR" | "BPCHAR" | "NAME" => json_opt(row.try_get::<Option<String>, _>(idx)),
        "UUID" => json_opt(
            row.try_get::<Option<uuid::Uuid>, _>(idx)
                .map(|o| o.map(|u| u.to_string())),
        ),
        "JSON" | "JSONB" => row
            .try_get::<Option<Value>, _>(idx)
            .unwrap_or(None)
            .unwrap_or(Value::Null),
        "TIMESTAMPTZ" => json_rfc3339(row.try_get::<Option<DateTime<Utc>>, _>(idx)),
        "TIMESTAMP" => json_display(row.try_get::<Option<NaiveDateTime>, _>(idx)),
        "DATE" => json_display(row.try_get::<Option<NaiveDate>, _>(idx)),
        "TIME" => json_display(row.try_get::<Option<NaiveTime>, _>(idx)),
        // Unknown type: fall back to the text representation Postgres would print.
        _ => json_opt(row.try_get::<Option<String>, _>(idx)),
    }
}

fn json_opt<T: Into<Value>>(res: Result<Option<T>, sqlx::Error>) -> Value {
    match res {
        Ok(Some(v)) => v.into(),
        Ok(None) => Value::Null,
        Err(_) => Value::Null,
    }
}

fn json_rfc3339(res: Result<Option<DateTime<Utc>>, sqlx::Error>) -> Value {
    match res {
        Ok(Some(ts)) => Value::String(ts.to_rfc3339()),
        _ => Value::Null,
    }
}

fn json_display<T: std::fmt::Display>(res: Result<Option<T>, sqlx::Error>) -> Value {
    match res {
        Ok(Some(v)) => Value::String(v.to_string()),
        _ => Value::Null,
    }
}
