//! Resolve and run the attached insight, returning a transformed response.

use nexus_spi::dto::insight::InsightRef;
use nexus_spi::dto::query::{ColumnSchema, QueryResponse, QueryStats, ResultColumnType};
use nexus_store::{extension_insight, insight};
use serde_json::{Map, Value};
use sqlx::PgPool;
use starter_spi::Error;

use crate::state::AppState;

/// Apply `req.insight` to `response` for `tenant`, returning the transformed
/// response. With no insight attached the response is returned unchanged. A
/// stored insight is resolved in `tenant` (None ⇒ the dev path, where only an
/// inline script is valid). The insight engine guarantees the row count never
/// grows, so no cap can be exceeded by the transform; stats are recomputed to
/// reflect the new rows, preserving the original `truncated` signal.
pub async fn apply_insight(
    state: &AppState,
    metadata: &PgPool,
    tenant: Option<&str>,
    insight: &InsightRef,
    response: QueryResponse,
) -> Result<QueryResponse, Error> {
    let script = resolve_script(metadata, tenant, insight).await?;
    let params = insight.params.clone().unwrap_or(Value::Null);
    let _ = state; // reserved for future per-tenant insight quotas

    let transformed = nexus_insights::run_insight_rows(script, response.rows, params)
        .await
        .map_err(|e| Error::Invalid {
            message: e.to_string(),
        })?;

    Ok(reshape(transformed, response.stats.elapsed_ms, response.stats.truncated))
}

/// Pick the script. Precedence: a stored tenant id (resolved + tenant-authorised
/// by RLS), then an extension-contributed name (the global registry, resolved
/// without a tenant — the script runs against the caller's own rows), then an
/// inline script. Absent all three is a caller error.
async fn resolve_script(
    metadata: &PgPool,
    tenant: Option<&str>,
    insight: &InsightRef,
) -> Result<String, Error> {
    if let Some(id) = insight.insight_id {
        let tenant = tenant.ok_or_else(|| Error::Invalid {
            message: "a stored insight requires a tenant context".into(),
        })?;
        return match insight::by_id(metadata, tenant, id).await? {
            Some(rec) => Ok(rec.script),
            None => Err(Error::NotFound {
                what: "insight not found".into(),
            }),
        };
    }
    if let Some(name) = &insight.insight_name {
        return match extension_insight::get_by_name(metadata, name).await? {
            Some(rec) => Ok(rec.script),
            None => Err(Error::NotFound {
                what: format!("extension insight `{name}` not found"),
            }),
        };
    }
    insight.script.clone().ok_or_else(|| Error::Invalid {
        message: "insight reference has neither an id, a name, nor an inline script".into(),
    })
}

/// Rebuild a `QueryResponse` from the transformed rows: derive the column schema
/// and byte count from the rows, keep the elapsed time, and preserve the upstream
/// `truncated` flag (the insight cannot truncate further — it never grows rows).
fn reshape(rows: Vec<Value>, elapsed_ms: u64, truncated: bool) -> QueryResponse {
    let columns = derive_columns(&rows);
    let byte_count = serde_json::to_vec(&rows).map(|b| b.len() as u64).unwrap_or(0);
    let row_count = rows.len() as u64;
    QueryResponse {
        columns,
        rows,
        stats: QueryStats {
            row_count,
            byte_count,
            elapsed_ms,
            truncated,
        },
    }
}

/// Infer a coarse column schema from the JSON rows. The first row's keys give the
/// column order; each column's type is the coarse type of its first non-null
/// value, defaulting to `String` for an all-null column.
fn derive_columns(rows: &[Value]) -> Vec<ColumnSchema> {
    let Some(first) = rows.iter().find_map(Value::as_object) else {
        return Vec::new();
    };
    first
        .keys()
        .map(|name| ColumnSchema {
            name: name.clone(),
            column_type: column_type(rows, name),
        })
        .collect()
}

/// Coarse type of `name` across `rows`: the first non-null value decides.
fn column_type(rows: &[Value], name: &str) -> ResultColumnType {
    for row in rows {
        if let Some(v) = row.as_object().and_then(|m: &Map<String, Value>| m.get(name)) {
            match v {
                Value::Null => continue,
                Value::Bool(_) => return ResultColumnType::Bool,
                Value::Number(n) if n.is_f64() => return ResultColumnType::Float,
                Value::Number(_) => return ResultColumnType::Int,
                Value::String(_) => return ResultColumnType::String,
                _ => return ResultColumnType::Other,
            }
        }
    }
    ResultColumnType::String
}
