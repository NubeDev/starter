//! Insert a JSON object row into a Postgres table with bound parameters.
//!
//! The write primitive behind the native `postgres` sink: columns are the
//! object's keys (quoted, so a reserved word is safe), values are bound rather
//! than interpolated, and a non-scalar value round-trips as its JSON text — a
//! lossless fallback for nested shapes against a text or jsonb column. Kept apart
//! from the sink so the SQL shaping is one named responsibility.

use serde_json::{Map, Value};
use sqlx::{PgPool, QueryBuilder};

use crate::core::{EngineError, EngineResult};

/// Insert one JSON object as a row in `table`. Returns [`EngineError::Sink`] on a
/// database error.
pub async fn insert_row(pool: &PgPool, table: &str, obj: &Map<String, Value>) -> EngineResult<()> {
    let cols: Vec<&String> = obj.keys().collect();
    let column_list = cols
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut qb =
        QueryBuilder::<sqlx::Postgres>::new(format!("INSERT INTO {table} ({column_list}) "));
    qb.push_values(std::iter::once(()), |mut b, _| {
        for c in &cols {
            bind_value(&mut b, &obj[*c]);
        }
    });
    qb.build()
        .execute(pool)
        .await
        .map_err(|e| EngineError::Sink(format!("postgres insert failed: {e}")))?;
    Ok(())
}

/// Bind one JSON value to the query, mapping JSON scalars to native Postgres
/// types and falling back to JSON text for arrays/objects.
fn bind_value<'a>(
    b: &mut sqlx::query_builder::Separated<'_, 'a, sqlx::Postgres, &'static str>,
    value: &'a Value,
) {
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
