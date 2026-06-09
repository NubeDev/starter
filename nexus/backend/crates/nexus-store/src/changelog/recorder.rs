//! Tenant-scoped [`ChangeRecorder`] over `nexus_changes`.
//!
//! Each [`ChangeRecorder::transaction`] opens a fresh tenant-bound transaction
//! (binding `app.tenant_id` via [`crate::tenant_tx`]) and assigns one `group_id`
//! shared by every row recorded inside the closure, so a multi-row mutation
//! undoes as one step. The `WITH CHECK` half of the RLS policy means an insert
//! whose `tenant_id` disagrees with the GUC is rejected by Postgres — the tenant
//! column cannot be spoofed past RLS.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeId, ChangeRecorder, ChangeTx, GroupId};
use starter_spi::{Error, Result};
use tokio::sync::Mutex;

use super::codec::{actor_columns, op_to_text};
use crate::tenant_tx;

/// Postgres-backed, tenant-scoped recorder. One per request is unnecessary —
/// it holds only a pool handle and the request's tenant, both cheap to clone.
#[derive(Clone)]
pub struct NexusRecorder {
    pool: PgPool,
    tenant_id: String,
}

impl NexusRecorder {
    /// Build a recorder pinned to `tenant_id`. Every row it writes carries this
    /// tenant and is RLS-checked against it.
    pub fn new(pool: PgPool, tenant_id: impl Into<String>) -> Self {
        Self {
            pool,
            tenant_id: tenant_id.into(),
        }
    }

    /// GDPR right-to-erasure for a **user subject**: tombstone the payloads of
    /// every change this subject authored (`actor_kind = 'user' AND actor_id =
    /// subject`), nulling `before`/`after`/`patch` while keeping `(id, at, op,
    /// group_id)` so replay counts, ordering, and undo grouping survive. The
    /// actor identity columns are kept too — an erasure request scrubs the
    /// *content* a user produced, not the audit fact that an action occurred (a
    /// regulator still needs to see "a now-erased user changed X at T"). Runs
    /// inside the tenant tx, so it can only reach this tenant's rows. Returns the
    /// number tombstoned.
    pub async fn forget_actor(&self, subject: &str) -> Result<u64> {
        let mut tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let affected = sqlx::query(
            r#"UPDATE nexus_changes
                  SET before = NULL, after = NULL, patch = NULL
                WHERE actor_kind = 'user' AND actor_id = $1"#,
        )
        .bind(subject)
        .execute(&mut *tx)
        .await
        .map_err(internal)?
        .rows_affected();
        tx.commit().await.map_err(internal)?;
        Ok(affected)
    }
}

/// In-transaction handle. Holds the tenant-bound transaction and the shared
/// `group_id`.
struct NexusChangeTx<'tx> {
    group_id: GroupId,
    tenant_id: String,
    tx: Mutex<Transaction<'tx, Postgres>>,
}

#[async_trait]
impl<'tx> ChangeTx for NexusChangeTx<'tx> {
    fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    async fn record(&self, mut ch: Change) -> Result<ChangeId> {
        let id = ChangeId(uuid::Uuid::now_v7().to_string());
        ch.id = id.clone();
        ch.group_id = self.group_id.clone();
        if ch.at.timestamp_nanos_opt().is_none() {
            ch.at = Utc::now();
        }

        let (actor_kind, actor_id, actor_meta) = actor_columns(&ch.actor);
        let resource_id = ch.resource.id.clone().ok_or_else(|| Error::Internal {
            source: "Change::resource.id must be Some for changelog rows".into(),
        })?;
        let op_text = op_to_text(&ch.op);
        let version_i64: Option<i64> = ch.resource_version.map(|v| v as i64);

        let mut guard = self.tx.lock().await;
        sqlx::query(
            r#"
            INSERT INTO nexus_changes (
                id, tenant_id, at, actor_kind, actor_id, actor_meta,
                resource_kind, resource_id, resource_owner, resource_version,
                op, before, after, patch, group_id, correlation
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16
            )
            "#,
        )
        .bind(&id.0)
        .bind(&self.tenant_id)
        .bind(ch.at)
        .bind(&actor_kind)
        .bind(&actor_id)
        .bind(&actor_meta)
        .bind(&ch.resource.kind)
        .bind(&resource_id)
        .bind(&ch.resource.owner)
        .bind(version_i64)
        .bind(&op_text)
        .bind(&ch.before)
        .bind(&ch.after)
        .bind(&ch.patch)
        .bind(&self.group_id.0)
        .bind(ch.correlation.as_ref().map(|t| t.0.clone()))
        .execute(&mut **guard)
        .await
        .map_err(internal)?;

        Ok(id)
    }
}

#[async_trait]
impl ChangeRecorder for NexusRecorder {
    async fn transaction<'a>(
        &'a self,
        f: Box<
            dyn for<'tx> FnOnce(
                    &'tx (dyn ChangeTx + 'tx),
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<()>> + Send + 'tx>,
                > + Send
                + 'a,
        >,
    ) -> Result<()> {
        let tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let handle = NexusChangeTx {
            group_id: GroupId(uuid::Uuid::now_v7().to_string()),
            tenant_id: self.tenant_id.clone(),
            tx: Mutex::new(tx),
        };

        let result = f(&handle).await;
        let NexusChangeTx { tx, .. } = handle;
        let tx = tx.into_inner();
        match result {
            Ok(()) => tx.commit().await.map_err(internal)?,
            Err(e) => {
                let _ = tx.rollback().await;
                return Err(e);
            }
        }
        Ok(())
    }

    async fn forget(&self, resource: &ResourceRef) -> Result<u64> {
        // Tombstone payloads while preserving row identity/order/grouping so
        // replay integrity survives (GDPR right-to-erasure). Runs inside the
        // tenant tx so it can only touch this tenant's rows.
        let mut tx = tenant_tx::begin(&self.pool, &self.tenant_id).await?;
        let affected = if let Some(id) = &resource.id {
            sqlx::query(
                r#"UPDATE nexus_changes
                      SET before = NULL, after = NULL, patch = NULL
                    WHERE resource_kind = $1 AND resource_id = $2"#,
            )
            .bind(&resource.kind)
            .bind(id)
            .execute(&mut *tx)
            .await
        } else {
            sqlx::query(
                r#"UPDATE nexus_changes
                      SET before = NULL, after = NULL, patch = NULL
                    WHERE resource_kind = $1"#,
            )
            .bind(&resource.kind)
            .execute(&mut *tx)
            .await
        }
        .map_err(internal)?
        .rows_affected();
        tx.commit().await.map_err(internal)?;
        Ok(affected)
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
