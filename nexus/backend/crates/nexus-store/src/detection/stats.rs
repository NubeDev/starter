//! Per-detection run stats: when it next runs + its findings by status.
//!
//! One tenant-scoped query joins the detection to its findings and aggregates —
//! cheap (the `(tenant_id, detection_id, status)` index covers the count), so
//! the list/editor can show "12 open · last spark 3m ago · next run in 4m"
//! without an N+1 over findings.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::DetectionStats;
use crate::tenant_tx;

/// Stats for detection `id` within the tenant. `Ok(None)` when the detection
/// does not exist (or belongs to another tenant — existence is not leaked).
pub async fn stats(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<DetectionStats>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // The detection row anchors the result (so a detection with zero findings
    // still returns a row, all-zero counts); the findings aggregate folds in via
    // a correlated subquery rather than a GROUP BY so the shape is one row.
    let row = sqlx::query(
        "SELECT \
            d.next_eval_at AS next_eval_at, \
            (SELECT max(at) FROM nexus_findings f WHERE f.detection_id = d.id) AS last_finding_at, \
            (SELECT count(*) FROM nexus_findings f WHERE f.detection_id = d.id AND f.status = 'open') AS open, \
            (SELECT count(*) FROM nexus_findings f WHERE f.detection_id = d.id AND f.status = 'acknowledged') AS acknowledged, \
            (SELECT count(*) FROM nexus_findings f WHERE f.detection_id = d.id AND f.status = 'resolved') AS resolved, \
            (SELECT count(*) FROM nexus_findings f WHERE f.detection_id = d.id) AS total \
         FROM nexus_detections d WHERE d.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.map(|r| DetectionStats {
        next_eval_at: r.get::<DateTime<Utc>, _>("next_eval_at"),
        last_finding_at: r.get::<Option<DateTime<Utc>>, _>("last_finding_at"),
        open: r.get::<i64, _>("open"),
        acknowledged: r.get::<i64, _>("acknowledged"),
        resolved: r.get::<i64, _>("resolved"),
        total: r.get::<i64, _>("total"),
    }))
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
