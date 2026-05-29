//! `PgWindowedFetcher` — per-bucket fetch impl for non-Timescale
//! Postgres tables.
//!
//! Mirrors the [`starter_store_warehouse::TimescaleWindowedFetcher`]
//! shape so callers that don't run TimescaleDB can still use the
//! `starter-windowed` machinery against regular Postgres time-bucketed
//! tables. The SQL template carries `$1` (bucket-start inclusive) and
//! `$2` (bucket-end exclusive) binds.

use crate::pool::Pool;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use starter_windowed::{Bucket, FetchError, RowSet, WindowedFetcher};

/// Postgres-backed windowed fetcher.
#[derive(Clone)]
pub struct PgWindowedFetcher {
    pool: Pool,
    sql: String,
}

impl PgWindowedFetcher {
    /// Construct from a connection [`Pool`] + SQL template.
    pub fn new(pool: Pool, sql: impl Into<String>) -> Self {
        Self {
            pool,
            sql: sql.into(),
        }
    }
}

#[async_trait]
impl WindowedFetcher<RowSet> for PgWindowedFetcher {
    async fn fetch_bucket(&self, bucket: Bucket) -> Result<RowSet, FetchError> {
        let from: DateTime<Utc> = bucket.start;
        let to: DateTime<Utc> = bucket.end;
        let rows = sqlx::query(&self.sql)
            .bind(from)
            .bind(to)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(|e| FetchError::Other(format!("sqlx: {e}")))?;
        let mut out: Vec<Value> = Vec::with_capacity(rows.len());
        for row in &rows {
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
