//! `TimescaleWindowedFetcher` — per-bucket fetch impl for Timescale
//! hypertables.
//!
//! Wraps a [`WarehouseClient`] + a SQL template. The template must
//! carry two positional bind sites for the bucket boundary —
//! `$1` (inclusive start) and `$2` (exclusive end) — and any other
//! parameters as later positional sites; the caller bakes the rest
//! of the params into a `Vec<serde_json::Value>` and passes it at
//! construction.
//!
//! Output: each row is rendered into a `serde_json::Value` map of
//! `{column_name -> value}` so the `RowSet` carries an
//! engine-agnostic JSON payload.

use crate::tsdb::client::WarehouseClient;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use starter_windowed::{Bucket, FetchError, RowSet, WindowedFetcher};

/// SQL template for one bucket. `{{from}}` and `{{to}}` placeholders
/// are substituted with `$1`/`$2` Postgres binds. (Hand substitution
/// — the bind values themselves are still parameterised so SQL
/// injection is impossible.)
#[derive(Clone)]
pub struct TimescaleWindowedFetcher {
    client: WarehouseClient,
    sql: String,
}

impl TimescaleWindowedFetcher {
    /// Construct from a client + SQL template. The template uses the
    /// literal placeholders `$1` (bucket start, inclusive) and `$2`
    /// (bucket end, exclusive). Example:
    ///
    /// ```sql
    /// SELECT bucket, value FROM histories
    ///   WHERE ts >= $1 AND ts < $2 ORDER BY ts
    /// ```
    pub fn new(client: WarehouseClient, sql: impl Into<String>) -> Self {
        Self {
            client,
            sql: sql.into(),
        }
    }
}

#[async_trait]
impl WindowedFetcher<RowSet> for TimescaleWindowedFetcher {
    async fn fetch_bucket(&self, bucket: Bucket) -> Result<RowSet, FetchError> {
        let from: DateTime<Utc> = bucket.start;
        let to: DateTime<Utc> = bucket.end;
        let rows = sqlx::query(&self.sql)
            .bind(from)
            .bind(to)
            .fetch_all(self.client.pool())
            .await
            .map_err(|e| FetchError::Other(format!("sqlx: {e}")))?;
        let mut out: Vec<Value> = Vec::with_capacity(rows.len());
        for row in &rows {
            // Render the row by introspecting columns; values are
            // re-encoded as JSON. Falls back to NULL when the column
            // type isn't directly representable.
            let mut obj = serde_json::Map::new();
            use sqlx::Column;
            use sqlx::Row;
            for (i, col) in row.columns().iter().enumerate() {
                let v: Option<Value> = row.try_get::<Value, _>(i).ok();
                obj.insert(col.name().to_string(), v.unwrap_or(Value::Null));
            }
            out.push(Value::Object(obj));
        }
        Ok(RowSet::new(out))
    }
}
