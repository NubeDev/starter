//! Catalog GC (W15).
//!
//! Terminal-state rows (`status IN ('quarantined','failed')`) older
//! than the configured retention horizon are pruned. `live` and
//! `pending` rows are never touched.

use serde::{Deserialize, Serialize};

use crate::pool::Pool;

/// Per-table reap counts.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GcReport {
    pub marts: u64,
    pub cleaners: u64,
    pub sandboxes: u64,
}

impl GcReport {
    pub fn total(&self) -> u64 {
        self.marts + self.cleaners + self.sandboxes
    }
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Run GC. `age_days` corresponds to the
/// `warehouse.catalog_gc_age_days` config key; the default is 90
/// per W15. Pass a large value (or skip the call) to disable.
pub async fn run(pool: &Pool, age_days: i32) -> Result<GcReport> {
    let mut report = GcReport::default();
    let interval = format!("{age_days} days");
    for (table, target) in [
        ("marts", &mut report.marts),
        ("cleaners", &mut report.cleaners),
    ] {
        let sql = format!(
            "DELETE FROM {table} \
             WHERE status IN ('quarantined','failed') \
               AND created_at < NOW() - $1::interval"
        );
        let res = sqlx::query(&sql).bind(&interval).execute(pool.sqlx()).await?;
        *target = res.rows_affected();
    }
    // Sandboxes use 'failed' for GC; 'promoted' is retained for
    // traceability per the SCOPE narrative.
    let res = sqlx::query(
        "DELETE FROM sandboxes \
         WHERE status = 'failed' \
           AND created_at < NOW() - $1::interval",
    )
    .bind(&interval)
    .execute(pool.sqlx())
    .await?;
    report.sandboxes = res.rows_affected();
    Ok(report)
}
