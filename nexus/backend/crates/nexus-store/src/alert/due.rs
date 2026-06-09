//! Claim the alert rules due for evaluation, across tenants.
//!
//! The scheduler is a system actor, so this calls the SECURITY DEFINER
//! `nexus_claim_due_alert_rules` function rather than a tenant-scoped query: it
//! is the one controlled cross-tenant read, and it advances each claimed rule's
//! `next_eval_at` atomically so a rule is not re-claimed before its interval. The
//! evaluator then loads and evaluates each returned rule under its own tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

/// A claimed rule: its id and owning tenant. Evaluation re-loads the full rule
/// under that tenant's RLS context.
#[derive(Debug, Clone)]
pub struct DueRule {
    pub id: Uuid,
    pub tenant_id: String,
}

/// Claim up to `batch` due rules, advancing their next evaluation time. Returns
/// the claimed (id, tenant) pairs; an empty vec means nothing is due.
pub async fn claim_due(pool: &PgPool, batch: i32) -> Result<Vec<DueRule>, Error> {
    let rows = sqlx::query("SELECT id, tenant_id FROM nexus_claim_due_alert_rules($1)")
        .bind(batch)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
    Ok(rows
        .iter()
        .map(|r| DueRule {
            id: r.get::<Uuid, _>("id"),
            tenant_id: r.get::<String, _>("tenant_id"),
        })
        .collect())
}
