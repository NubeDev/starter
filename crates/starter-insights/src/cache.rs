//! Derivation cache — materialisation tier 3 (Insights SCOPE).
//!
//! When a derivation rule declares `persist: true` in its
//! [`starter_spi::insights::RuleSchema`], the engine writes the
//! emitted `Dataset` to this cache on every successful invocation,
//! keyed on `(rule_id, window)`. Downstream nodes (and the
//! frontend's chart endpoint) read from the cache rather than
//! re-deriving.
//!
//! Invalidation seams:
//! - **Rule version bump** — `invalidate_rule_version(ns, name)`
//!   wipes all entries for any major of the rule (a major bump is
//!   the registry's "breaking change" marker).
//! - **Admin `cache.invalidate`** — `invalidate(rule_id)` wipes a
//!   single `(namespace, name, major)`.
//!
//! A non-deterministic derivation with `persist: true` is a bug;
//! the backfill determinism smoke (R-ins-2) catches it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use starter_spi::insights::{Dataset, RuleId, RuleSchema};
use starter_store_sqlite::pool::Pool;
use thiserror::Error;

/// Cache error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DerivationCacheError {
    /// Driver-level failure.
    #[error("derivation cache backend: {0}")]
    Backend(String),
    /// Serialisation failure.
    #[error("derivation cache serialisation: {0}")]
    Serde(String),
}

/// Serialised payload — schema, snapshot rows, coverage, tz, window.
/// Kept in a single JSON blob; the cache is value-keyed by
/// `(rule_id, window)` so the payload shape is opaque to the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachePayload {
    schema_columns: Vec<String>,
    rows: Vec<serde_json::Value>,
    coverage: starter_spi::insights::Coverage,
    tz: String,
    window_start_ms: Option<i64>,
    window_end_ms: Option<i64>,
}

impl CachePayload {
    fn from_dataset(ds: &Dataset) -> Self {
        Self {
            schema_columns: ds.schema.columns.clone(),
            rows: ds.rows.snapshot(),
            coverage: ds.coverage.clone(),
            tz: ds.tz.as_str().to_owned(),
            window_start_ms: ds.window.as_ref().map(|w| w.start.timestamp_millis()),
            window_end_ms: ds.window.as_ref().map(|w| w.end.timestamp_millis()),
        }
    }

    fn into_dataset(self) -> Dataset {
        use starter_spi::insights::{DatasetSchema, TimeZoneId, VecDatasetRows, Window};
        use std::sync::Arc;
        let window = match (self.window_start_ms, self.window_end_ms) {
            (Some(s), Some(e)) => Some(Window::new(
                DateTime::from_timestamp_millis(s).unwrap_or_else(Utc::now),
                DateTime::from_timestamp_millis(e).unwrap_or_else(Utc::now),
            )),
            _ => None,
        };
        Dataset::from_parts(
            DatasetSchema::new(self.schema_columns),
            Arc::new(VecDatasetRows::new(self.rows)),
            self.coverage,
            TimeZoneId::new(self.tz),
            window,
        )
    }
}

/// Derivation cache bound to a SQLite pool.
#[derive(Clone)]
pub struct DerivationCache {
    pool: Pool,
}

impl DerivationCache {
    /// Construct a cache over an existing pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Whether `schema` opts the rule into persistence. Helper for
    /// the engine's "should I write this?" check.
    pub fn should_persist(schema: &RuleSchema) -> bool {
        schema.persist
    }

    /// Insert / overwrite a cache entry for `(rule_id, window)`.
    ///
    /// `window_start_ms`/`window_end_ms` should be the *cache key*
    /// window (typically the upstream `align`/`window.*` boundary).
    /// Inline rules without a window key on `0..0`; the value is
    /// readable but the cache is effectively single-entry.
    pub async fn put(
        &self,
        rule_id: &RuleId,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
        ds: &Dataset,
    ) -> Result<(), DerivationCacheError> {
        let payload = CachePayload::from_dataset(ds);
        let body = serde_json::to_string(&payload)
            .map_err(|e| DerivationCacheError::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO derivation_cache \
                 (rule_namespace, rule_name, rule_major, \
                  window_start_ms, window_end_ms, payload_json, written_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(rule_namespace, rule_name, rule_major, window_start_ms) \
             DO UPDATE SET payload_json = excluded.payload_json, \
                           window_end_ms = excluded.window_end_ms, \
                           written_ms = excluded.written_ms",
        )
        .bind(&rule_id.namespace)
        .bind(&rule_id.name)
        .bind(rule_id.major as i64)
        .bind(window_start.timestamp_millis())
        .bind(window_end.timestamp_millis())
        .bind(body)
        .bind(Utc::now().timestamp_millis())
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| DerivationCacheError::Backend(e.to_string()))?;
        Ok(())
    }

    /// Read a cached dataset for `(rule_id, window_start)`. Returns
    /// `None` for a cache miss.
    pub async fn get(
        &self,
        rule_id: &RuleId,
        window_start: DateTime<Utc>,
    ) -> Result<Option<Dataset>, DerivationCacheError> {
        let row = sqlx::query(
            "SELECT payload_json FROM derivation_cache \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3 \
               AND window_start_ms=?4",
        )
        .bind(&rule_id.namespace)
        .bind(&rule_id.name)
        .bind(rule_id.major as i64)
        .bind(window_start.timestamp_millis())
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(|e| DerivationCacheError::Backend(e.to_string()))?;
        let Some(row) = row else { return Ok(None) };
        let body: String = row.get("payload_json");
        let payload: CachePayload =
            serde_json::from_str(&body).map_err(|e| DerivationCacheError::Serde(e.to_string()))?;
        Ok(Some(payload.into_dataset()))
    }

    /// Invalidate every entry for a specific `(namespace, name,
    /// major)`. Admin `cache.invalidate` calls this.
    pub async fn invalidate(&self, rule_id: &RuleId) -> Result<u64, DerivationCacheError> {
        let r = sqlx::query(
            "DELETE FROM derivation_cache \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3",
        )
        .bind(&rule_id.namespace)
        .bind(&rule_id.name)
        .bind(rule_id.major as i64)
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| DerivationCacheError::Backend(e.to_string()))?;
        Ok(r.rows_affected())
    }

    /// Invalidate every entry across every major of a rule —
    /// called on a `RuleId` major bump. "Nothing auto-rewarms" is
    /// load-bearing here: the next scheduled tick repopulates.
    pub async fn invalidate_rule_version(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<u64, DerivationCacheError> {
        let r = sqlx::query(
            "DELETE FROM derivation_cache \
             WHERE rule_namespace=?1 AND rule_name=?2",
        )
        .bind(namespace)
        .bind(name)
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| DerivationCacheError::Backend(e.to_string()))?;
        Ok(r.rows_affected())
    }

    /// Test helper: row count.
    pub async fn count(&self) -> Result<i64, DerivationCacheError> {
        let n: i64 = sqlx::query("SELECT COUNT(*) AS n FROM derivation_cache")
            .fetch_one(self.pool.sqlx())
            .await
            .map_err(|e| DerivationCacheError::Backend(e.to_string()))?
            .get("n");
        Ok(n)
    }
}
