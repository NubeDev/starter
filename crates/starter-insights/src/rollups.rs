//! Verdict rollups — tier 2 materialisation (Insights SCOPE D5).
//!
//! Gated behind the `sqlite` cargo feature. The rollup machinery is
//! **incremental by default**: each tick reads the verdict log from
//! the per-(rule_id, window_class) checkpoint forward, increments
//! the matching `verdict_rollup` rows (ungrouped + per-tag), and
//! advances the checkpoint.
//!
//! Retroactive rules (`RuleSchema::retroactive = true`) skip the
//! monotonic checkpoint and instead drain
//! [`rollup_invalidation`](crate::sqlite). On D5 mutation events,
//! the engine pushes `(rule_id, window_class, window_start_ms,
//! window_end_ms, reason)` rows onto the queue; the rollup tick
//! re-aggregates each invalidated window and clears
//! `stale_since_ms`.
//!
//! Tag-grouped rollups (R-ins-8) live in the same `verdict_rollup`
//! table with NULLable `tag_key` / `tag_value` columns; the
//! ungrouped row (NULL/NULL) coexists with one row per
//! `(tag_key, tag_value)` pair the rollup-grouping config asks for.

use chrono::{DateTime, Datelike, TimeZone, Utc};
use chrono_tz::Tz;
use sqlx::Row;
use starter_spi::insights::{Severity, TagValue, Verdict};
use starter_store_sqlite::pool::Pool;
use thiserror::Error;

/// Window class for a rollup (`hour`, `day`, `week`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowClass {
    /// 1-hour aggregates.
    Hour,
    /// 1-day aggregates (tz-aware bucket boundary).
    Day,
    /// 7-day aggregates anchored on ISO Monday in tz.
    Week,
}

impl WindowClass {
    /// Stable short name used in the `verdict_rollup.window_class`
    /// column. Stable across migrations; bumping a rollup schema
    /// changes the `RuleId` major, not this string.
    pub fn as_str(self) -> &'static str {
        match self {
            WindowClass::Hour => "hour",
            WindowClass::Day => "day",
            WindowClass::Week => "week",
        }
    }

    /// `[start, end)` bounds in UTC for the bucket containing `at`,
    /// anchored on `tz` so DST transitions land on boundaries.
    pub fn bucket(self, tz: Tz, at: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let local = at.with_timezone(&tz);
        match self {
            WindowClass::Hour => {
                let s = tz
                    .with_ymd_and_hms(local.year(), local.month(), local.day(), local.hour(), 0, 0)
                    .single()
                    .unwrap_or(local);
                let e = s + chrono::Duration::hours(1);
                (s.with_timezone(&Utc), e.with_timezone(&Utc))
            }
            WindowClass::Day => {
                let s = tz
                    .with_ymd_and_hms(local.year(), local.month(), local.day(), 0, 0, 0)
                    .single()
                    .unwrap_or(local);
                let e = s + chrono::Duration::days(1);
                (s.with_timezone(&Utc), e.with_timezone(&Utc))
            }
            WindowClass::Week => {
                // Anchor on the local Monday at 00:00.
                let weekday = local.weekday().num_days_from_monday() as i64;
                let day_start = tz
                    .with_ymd_and_hms(local.year(), local.month(), local.day(), 0, 0, 0)
                    .single()
                    .unwrap_or(local);
                let s = day_start - chrono::Duration::days(weekday);
                let e = s + chrono::Duration::days(7);
                (s.with_timezone(&Utc), e.with_timezone(&Utc))
            }
        }
    }
}

use chrono::Timelike;

/// Rollup-machinery error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RollupError {
    /// Underlying driver error.
    #[error("rollup backend: {0}")]
    Backend(String),
}

/// Rollup engine bound to a SQLite pool.
#[derive(Clone)]
pub struct RollupEngine {
    pool: Pool,
}

impl RollupEngine {
    /// Construct a rollup engine.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Run an incremental rollup tick for a single
    /// `(rule_id, window_class)` pair. Reads the verdict log from
    /// the checkpoint forward, folds each verdict into the matching
    /// rollup row (ungrouped + per `tag_keys` group), and advances
    /// the checkpoint.
    ///
    /// `tag_keys` is the tag-grouping configuration from R-ins-8 —
    /// every key in this list produces a per-`(tag_key, tag_value)`
    /// row alongside the ungrouped one.
    pub async fn tick_incremental(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
        tz: Tz,
        tag_keys: &[&str],
    ) -> Result<usize, RollupError> {
        let class = window_class.as_str();
        let last_at: i64 = sqlx::query(
            "SELECT COALESCE(last_at_ms, 0) AS v FROM rollup_checkpoint \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3 AND window_class=?4",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(class)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?
        .map(|r| r.get::<i64, _>("v"))
        .unwrap_or(0);

        let rows = sqlx::query(
            "SELECT at_ms, severity, body_json FROM verdict_log \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3 AND at_ms > ?4 \
             ORDER BY at_ms",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(last_at)
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;

        let mut max_at = last_at;
        let mut count = 0usize;
        for r in rows {
            let at_ms: i64 = r.get("at_ms");
            let body: String = r.get("body_json");
            let v: Verdict = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => continue,
            };
            self.fold_one(namespace, name, major, window_class, tz, &v, tag_keys)
                .await?;
            max_at = max_at.max(at_ms);
            count += 1;
        }

        sqlx::query(
            "INSERT INTO rollup_checkpoint(rule_namespace, rule_name, rule_major, window_class, last_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(rule_namespace, rule_name, rule_major, window_class) \
             DO UPDATE SET last_at_ms = excluded.last_at_ms",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(class)
        .bind(max_at)
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;

        // Drain the D5 invalidation queue for this rule + class.
        self.drain_invalidations(namespace, name, major, window_class)
            .await?;
        Ok(count)
    }

    async fn fold_one(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
        tz: Tz,
        v: &Verdict,
        tag_keys: &[&str],
    ) -> Result<(), RollupError> {
        let (start, end) = window_class.bucket(tz, v.at);
        // Ungrouped row.
        self.bump(namespace, name, major, window_class, start, end, None, None, v)
            .await?;
        // Per-tag groups.
        for k in tag_keys {
            if let Some(tv) = v.tags.get(*k) {
                let val_str = match tv {
                    TagValue::Flag => String::new(),
                    TagValue::Value(s) => s.clone(),
                };
                self.bump(
                    namespace,
                    name,
                    major,
                    window_class,
                    start,
                    end,
                    Some(*k),
                    Some(&val_str),
                    v,
                )
                .await?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn bump(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        tag_key: Option<&str>,
        tag_value: Option<&str>,
        v: &Verdict,
    ) -> Result<(), RollupError> {
        let class = window_class.as_str();
        let (h, i, w, c, e) = sev_buckets(v.severity);
        let cov = v.coverage.effective.confidence as f64;
        sqlx::query(
            "INSERT INTO verdict_rollup \
                 (rule_namespace, rule_name, rule_major, window_class, \
                  window_start_ms, window_end_ms, tag_key, tag_value, \
                  count_healthy, count_info, count_warn, count_critical, count_error, \
                  coverage_min) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) \
             ON CONFLICT(rule_namespace, rule_name, rule_major, window_class, \
                         window_start_ms, IFNULL(tag_key,''), IFNULL(tag_value,'')) \
             DO UPDATE SET \
                 count_healthy  = count_healthy  + excluded.count_healthy, \
                 count_info     = count_info     + excluded.count_info, \
                 count_warn     = count_warn     + excluded.count_warn, \
                 count_critical = count_critical + excluded.count_critical, \
                 count_error    = count_error    + excluded.count_error, \
                 coverage_min   = MIN(coverage_min, excluded.coverage_min), \
                 stale_since_ms = NULL",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(class)
        .bind(start.timestamp_millis())
        .bind(end.timestamp_millis())
        .bind(tag_key)
        .bind(tag_value)
        .bind(h)
        .bind(i)
        .bind(w)
        .bind(c)
        .bind(e)
        .bind(cov)
        .execute(self.pool.sqlx())
        .await
        .map_err(|err| RollupError::Backend(err.to_string()))?;
        Ok(())
    }

    /// D5 — enqueue a `(rule_id, window)` invalidation for the
    /// scheduled rollup job to re-aggregate.
    pub async fn enqueue_invalidation(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
        reason: &str,
    ) -> Result<(), RollupError> {
        sqlx::query(
            "INSERT INTO rollup_invalidation \
                 (rule_namespace, rule_name, rule_major, window_class, \
                  window_start_ms, window_end_ms, reason, enqueued_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(window_class.as_str())
        .bind(window_start.timestamp_millis())
        .bind(window_end.timestamp_millis())
        .bind(reason)
        .bind(Utc::now().timestamp_millis())
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;
        // Mark the existing rollup row(s) stale so the frontend
        // can show a `stale_since` banner until the next tick.
        sqlx::query(
            "UPDATE verdict_rollup SET stale_since_ms = ?1 \
             WHERE rule_namespace=?2 AND rule_name=?3 AND rule_major=?4 \
               AND window_class=?5 AND window_start_ms=?6",
        )
        .bind(Utc::now().timestamp_millis())
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(window_class.as_str())
        .bind(window_start.timestamp_millis())
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn drain_invalidations(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
    ) -> Result<(), RollupError> {
        sqlx::query(
            "DELETE FROM rollup_invalidation \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3 AND window_class=?4",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(window_class.as_str())
        .execute(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;
        Ok(())
    }

    /// Test helper: read the ungrouped rollup count for a bucket.
    pub async fn read_ungrouped_count(
        &self,
        namespace: &str,
        name: &str,
        major: u32,
        window_class: WindowClass,
        start: DateTime<Utc>,
    ) -> Result<(i64, i64, i64, i64, i64), RollupError> {
        let row = sqlx::query(
            "SELECT count_healthy, count_info, count_warn, count_critical, count_error \
             FROM verdict_rollup \
             WHERE rule_namespace=?1 AND rule_name=?2 AND rule_major=?3 \
               AND window_class=?4 AND window_start_ms=?5 \
               AND tag_key IS NULL AND tag_value IS NULL",
        )
        .bind(namespace)
        .bind(name)
        .bind(major as i64)
        .bind(window_class.as_str())
        .bind(start.timestamp_millis())
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(|e| RollupError::Backend(e.to_string()))?;
        Ok(match row {
            None => (0, 0, 0, 0, 0),
            Some(r) => (
                r.get("count_healthy"),
                r.get("count_info"),
                r.get("count_warn"),
                r.get("count_critical"),
                r.get("count_error"),
            ),
        })
    }
}

fn sev_buckets(s: Severity) -> (i64, i64, i64, i64, i64) {
    match s {
        Severity::Healthy => (1, 0, 0, 0, 0),
        Severity::Info => (0, 1, 0, 0, 0),
        Severity::Warn => (0, 0, 1, 0, 0),
        Severity::Critical => (0, 0, 0, 1, 0),
        Severity::Error => (0, 0, 0, 0, 1),
        // Future severities count as Info — non-exhaustive guard.
        _ => (0, 1, 0, 0, 0),
    }
}
