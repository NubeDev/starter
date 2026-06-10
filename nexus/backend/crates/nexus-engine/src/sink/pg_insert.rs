//! Insert JSON object rows into a Postgres table with bound parameters, typed by
//! the stream's Arrow schema.
//!
//! The write primitive behind the native `postgres` sink. Columns are the
//! object's keys (quoted, so a reserved word is safe) and values are bound rather
//! than interpolated. Binding is **schema-driven**: the target column's Arrow
//! [`DataType`] decides the Postgres type a value binds as — a string under a
//! `Timestamp` column binds as `timestamptz`, the same string under a `Utf8`
//! column stays `text`. This is the difference from content-guessing (parsing
//! every string to see if it "looks like" a date), which silently mistypes a
//! text column that happens to hold a date-shaped value. The schema is the single
//! source of truth, set once by `json_to_arrow` (declared or inferred) upstream.
//!
//! A column with no entry in the schema map (e.g. a row key absent from the
//! batch schema) falls back to JSON-scalar binding, and a non-scalar value
//! round-trips as its JSON text — a lossless fallback for nested shapes against a
//! text or jsonb column.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use datafusion::arrow::datatypes::{DataType, SchemaRef};
use serde_json::{Map, Value};
use sqlx::{PgPool, QueryBuilder};

use crate::core::{EngineError, EngineResult};

/// Column-name → Arrow [`DataType`] for the stream, derived once from the batch
/// schema. Binding consults this so a value's Postgres type follows the declared
/// column type, not the value's JSON shape.
pub type ColumnTypes = HashMap<String, DataType>;

/// Build a [`ColumnTypes`] map from an Arrow schema.
pub fn column_types(schema: &SchemaRef) -> ColumnTypes {
    schema
        .fields()
        .iter()
        .map(|f| (f.name().clone(), f.data_type().clone()))
        .collect()
}

/// Insert one JSON object as a row in `table`, binding each value by its column's
/// Arrow type from `types`. `conflict` is an optional trailing `ON CONFLICT …`
/// clause (built by the sink from its configured policy). Returns
/// [`EngineError::Sink`] on a database error.
pub async fn insert_row(
    pool: &PgPool,
    table: &str,
    obj: &Map<String, Value>,
    types: &ColumnTypes,
    conflict: Option<&str>,
) -> EngineResult<()> {
    let cols: Vec<&String> = obj.keys().collect();
    let column_list = cols
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut qb =
        QueryBuilder::<sqlx::Postgres>::new(format!("INSERT INTO \"{table}\" ({column_list}) "));
    qb.push_values(std::iter::once(()), |mut b, _| {
        for c in &cols {
            bind_value(&mut b, &obj[*c], types.get(*c));
        }
    });
    if let Some(clause) = conflict {
        qb.push(format!(" {clause}"));
    }
    qb.build()
        .execute(pool)
        .await
        .map_err(|e| EngineError::Sink(format!("postgres insert failed: {e}")))?;
    Ok(())
}

/// Bind one JSON value to the query. When the column's Arrow type is known and is
/// a timestamp/date, an RFC3339 string binds as a `timestamptz`; otherwise the
/// value binds by its JSON scalar shape (text/bool/int/float), with arrays and
/// objects round-tripping as JSON text.
fn bind_value<'a>(
    b: &mut sqlx::query_builder::Separated<'_, 'a, sqlx::Postgres, &'static str>,
    value: &'a Value,
    column_type: Option<&DataType>,
) {
    // A string under a temporal column binds as a real timestamptz so the target
    // column can be `timestamptz` rather than forced to `text`. `json_to_arrow`
    // renders an Arrow `Timestamp` to JSON without a timezone suffix
    // (`2024-05-29T16:26:40`), so try a naive parse (assume UTC) as well as a full
    // RFC3339 one. Parsing failure falls through to a plain text bind so a
    // malformed value surfaces as a clear type error at the DB, not a silent drop.
    if let (Value::String(s), Some(dt)) = (value, column_type) {
        if is_temporal(dt) {
            if let Some(parsed) = parse_timestamp(s) {
                b.push_bind(parsed);
                return;
            }
        }
    }

    match value {
        Value::Null => {
            b.push_bind(None::<String>);
        }
        Value::Bool(v) => {
            b.push_bind(*v);
        }
        Value::Number(n) if n.is_i64() => {
            b.push_bind(n.as_i64().unwrap());
        }
        Value::Number(n) if n.is_u64() => {
            b.push_bind(n.as_u64().unwrap() as i64);
        }
        Value::Number(n) => {
            b.push_bind(n.as_f64().unwrap_or(0.0));
        }
        Value::String(s) => {
            b.push_bind(s.clone());
        }
        other => {
            b.push_bind(other.to_string());
        }
    }
}

/// Whether an Arrow type maps to a Postgres temporal column (timestamptz).
fn is_temporal(dt: &DataType) -> bool {
    matches!(
        dt,
        DataType::Timestamp(_, _) | DataType::Date32 | DataType::Date64
    )
}

/// Parse a timestamp string to a UTC instant. Accepts a full RFC3339 string
/// (with offset/`Z`) or a timezone-less form as produced by `json_to_arrow`'s
/// Arrow→JSON render (`2024-05-29T16:26:40[.ffffff]`), which is treated as UTC.
fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(naive.and_utc());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_with_offset() {
        assert!(parse_timestamp("2024-05-29T16:26:40Z").is_some());
        assert!(parse_timestamp("2024-05-29T16:26:40+02:00").is_some());
    }

    #[test]
    fn parses_timezoneless_arrow_render_as_utc() {
        // The exact shape json_to_arrow emits for Timestamp(Microsecond, None).
        let dt = parse_timestamp("2024-05-29T16:26:40").expect("naive parse");
        assert_eq!(dt.to_rfc3339(), "2024-05-29T16:26:40+00:00");
        assert!(parse_timestamp("2024-05-29T16:26:40.123456").is_some());
    }

    #[test]
    fn rejects_non_timestamp_string() {
        assert!(parse_timestamp("site-001").is_none());
    }

    #[test]
    fn is_temporal_matches_timestamp_and_date_only() {
        assert!(is_temporal(&DataType::Timestamp(
            datafusion::arrow::datatypes::TimeUnit::Microsecond,
            None
        )));
        assert!(is_temporal(&DataType::Date32));
        assert!(!is_temporal(&DataType::Utf8));
        assert!(!is_temporal(&DataType::Int64));
    }
}
