//! Claim the detections due to run, across tenants.
//!
//! Mirrors [`crate::alert::due`]: the runner is a system actor, so this calls
//! the SECURITY DEFINER `nexus_claim_due_detections` function rather than a
//! tenant-scoped query. It is the one controlled cross-tenant read, and it
//! advances each claimed detection's `next_eval_at` atomically so a detection is
//! not re-claimed before its interval. The runner then loads and runs each
//! returned detection under its own tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

/// A claimed detection: its id and owning tenant. The run re-loads the full
/// detection under that tenant's RLS context.
#[derive(Debug, Clone)]
pub struct DueDetection {
    pub id: Uuid,
    pub tenant_id: String,
}

/// Claim up to `batch` due detections, advancing their next run time. Returns
/// the claimed (id, tenant) pairs; an empty vec means nothing is due.
pub async fn claim_due(pool: &PgPool, batch: i32) -> Result<Vec<DueDetection>, Error> {
    let rows = sqlx::query("SELECT id, tenant_id FROM nexus_claim_due_detections($1)")
        .bind(batch)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
    Ok(rows
        .iter()
        .map(|r| DueDetection {
            id: r.get::<Uuid, _>("id"),
            tenant_id: r.get::<String, _>("tenant_id"),
        })
        .collect())
}
