//! SQLite-backed verdict log + tag index (Phase 1 persistence).
//!
//! Gated behind the `sqlite` cargo feature on `starter-insights`.
//! No rollups, no derivation cache — those land in Phase 2+. The
//! schema is intentionally minimal:
//!
//! - `verdict_log(id, rule_namespace, rule_name, rule_major, at,
//!    severity, summary, body_json)` — append-only.
//! - `verdict_tag(verdict_id, key, value)` — composite index on
//!    `(key, value)`. Implements the R-ins-8 indexed tag lookup
//!    behind the frontend filter contract.
//!
//! Note: this module deliberately exposes the migrator and the
//! store via the same `starter-store-sqlite::MigrationSource`
//! shape every other persistence module uses (`SKILL_APPROVALS_*`,
//! `FLOW_*`). The schema lives next to the source for context;
//! `sqlx::migrate!` reads from `migrations/insights/`.

use sqlx::Row;
use starter_spi::insights::{TagValue, Verdict};
use starter_store_sqlite::pool::Pool;
use thiserror::Error;

/// SQL migrator for the Phase 1 insights schema.
pub static INSIGHTS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/insights");

/// `MigrationSource` consumers chain into `migrate(pool)` on boot.
pub const INSIGHTS_MIGRATION_SOURCE: starter_store_sqlite::migrate::MigrationSource =
    starter_store_sqlite::migrate::MigrationSource {
        name: "insights",
        migrator: &INSIGHTS_MIGRATOR,
    };

/// Verdict-store error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VerdictStoreError {
    /// Underlying driver error.
    #[error("verdict store backend: {0}")]
    Backend(String),
    /// Serialisation error.
    #[error("verdict serialisation: {0}")]
    Serde(String),
}

/// SQLite verdict log + tag index.
#[derive(Clone)]
pub struct VerdictStore {
    pool: Pool,
}

impl VerdictStore {
    /// Construct a [`VerdictStore`] over an existing pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Append a verdict + its tag index entries.
    pub async fn append(&self, verdict: &Verdict) -> Result<i64, VerdictStoreError> {
        let body = serde_json::to_string(verdict)
            .map_err(|e| VerdictStoreError::Serde(e.to_string()))?;
        let severity = format!("{:?}", verdict.severity).to_lowercase();
        let at_ms = verdict.at.timestamp_millis();

        let mut tx = self
            .pool
            .sqlx()
            .begin()
            .await
            .map_err(|e| VerdictStoreError::Backend(e.to_string()))?;

        let id: i64 = sqlx::query(
            "INSERT INTO verdict_log \
                 (rule_namespace, rule_name, rule_major, at_ms, severity, summary, body_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             RETURNING id",
        )
        .bind(&verdict.rule_id.namespace)
        .bind(&verdict.rule_id.name)
        .bind(verdict.rule_id.major as i64)
        .bind(at_ms)
        .bind(&severity)
        .bind(&verdict.summary)
        .bind(&body)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| VerdictStoreError::Backend(e.to_string()))?
        .get("id");

        for (k, v) in verdict.tags.iter() {
            let val = match v {
                TagValue::Flag => None,
                TagValue::Value(s) => Some(s.as_str()),
            };
            sqlx::query("INSERT INTO verdict_tag (verdict_id, key, value) VALUES (?1, ?2, ?3)")
                .bind(id)
                .bind(k)
                .bind(val)
                .execute(&mut *tx)
                .await
                .map_err(|e| VerdictStoreError::Backend(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| VerdictStoreError::Backend(e.to_string()))?;
        Ok(id)
    }

    /// Count rows in the verdict log. Test helper.
    pub async fn count(&self) -> Result<i64, VerdictStoreError> {
        let n: i64 = sqlx::query("SELECT COUNT(*) AS n FROM verdict_log")
            .fetch_one(self.pool.sqlx())
            .await
            .map_err(|e| VerdictStoreError::Backend(e.to_string()))?
            .get("n");
        Ok(n)
    }

    /// List verdict ids matching `tags[key] = value`. Phase 1
    /// surface for the frontend tag-filter contract (R-ins-8).
    pub async fn list_ids_by_tag(
        &self,
        key: &str,
        value: Option<&str>,
    ) -> Result<Vec<i64>, VerdictStoreError> {
        let rows = if let Some(v) = value {
            sqlx::query(
                "SELECT verdict_id FROM verdict_tag WHERE key = ?1 AND value = ?2 ORDER BY verdict_id",
            )
            .bind(key)
            .bind(v)
            .fetch_all(self.pool.sqlx())
            .await
        } else {
            sqlx::query(
                "SELECT verdict_id FROM verdict_tag WHERE key = ?1 AND value IS NULL ORDER BY verdict_id",
            )
            .bind(key)
            .fetch_all(self.pool.sqlx())
            .await
        }
        .map_err(|e| VerdictStoreError::Backend(e.to_string()))?;
        Ok(rows.into_iter().map(|r| r.get::<i64, _>("verdict_id")).collect())
    }
}
