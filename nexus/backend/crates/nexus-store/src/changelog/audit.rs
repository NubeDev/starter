//! Audit-only changelog rows for global-config mutations.
//!
//! Most `nexus_changes` rows are *reversible* resource mutations recorded via
//! [`NexusRecorder`](super::NexusRecorder) + the `ReversibleRegistry`. Some
//! admin actions are **global config**, not per-tenant reversible resources —
//! WS-14 extension enable/disable/install/uninstall is the first such case
//! (WS-12 calls these out as audit-only, like the file-pack kinds). They have no
//! `before`/`after` snapshot to undo and no per-tenant row to key on, but they
//! must still leave an attributed audit trail.
//!
//! [`record_audit`] writes one such row: an [`Op::Custom`] entry under the
//! acting admin's tenant (the audit ledger is tenant-scoped + RLS, so the entry
//! lives in the admin's own tenant view — the natural place an operator looks
//! for "what did I change"). The `resource_kind`/`resource_id` name the global
//! object acted on (e.g. `extension` / `com.nubeio.notes`); `op` carries the
//! verb (`enabled`, `disabled`, …). This is observational only — no undo
//! cursor, no Reversible registration.

use chrono::Utc;
use sqlx::PgPool;
use starter_spi::changelog::{Actor, Op};
use starter_spi::Result;

use crate::tenant_tx;

use super::codec::{actor_columns, op_to_text};

/// Record a global-config audit entry under `tenant_id`, attributed to `actor`.
///
/// `op` is the verb (e.g. `Op::Custom("enabled".into())`); `resource_kind` /
/// `resource_id` name the global object (e.g. `"extension"` /
/// `"com.nubeio.notes"`). Returns the new row id. A fresh `group_id` is minted
/// per call — these entries are never grouped, since they are not undoable.
///
/// Runs inside a [`tenant_tx`] so the `app.tenant_id` GUC is bound for the RLS
/// policy; the row lands in the acting admin's tenant view.
pub async fn record_audit(
    pool: &PgPool,
    tenant_id: &str,
    actor: &Actor,
    op: &Op,
    resource_kind: &str,
    resource_id: &str,
) -> Result<String> {
    let id = uuid::Uuid::now_v7().to_string();
    let group_id = uuid::Uuid::now_v7().to_string();
    let (actor_kind, actor_id, actor_meta) = actor_columns(actor);
    let op_text = op_to_text(op);
    let at = Utc::now();

    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        r#"
        INSERT INTO nexus_changes (
            id, tenant_id, at, actor_kind, actor_id, actor_meta,
            resource_kind, resource_id, resource_owner, resource_version,
            op, before, after, patch, group_id, correlation
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, NULL, NULL,
            $9, NULL, NULL, NULL, $10, NULL
        )
        "#,
    )
    .bind(&id)
    .bind(tenant_id)
    .bind(at)
    .bind(&actor_kind)
    .bind(&actor_id)
    .bind(&actor_meta)
    .bind(resource_kind)
    .bind(resource_id)
    .bind(&op_text)
    .bind(&group_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| starter_spi::Error::Internal {
        source: Box::new(e),
    })?;
    tx.commit().await.map_err(|e| starter_spi::Error::Internal {
        source: Box::new(e),
    })?;

    Ok(id)
}
