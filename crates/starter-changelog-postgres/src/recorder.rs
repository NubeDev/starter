//! [`ChangeRecorder`] / [`ChangeTx`] impls over Postgres.

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeId, ChangeRecorder, ChangeTx, GroupId};
use starter_spi::{Error, Result};
use starter_store_postgres::Pool;
use tokio::sync::Mutex;

use crate::codec::{actor_columns, op_to_text};
use crate::ids::{new_change_id, new_group_id};

/// Postgres-backed [`ChangeRecorder`].
#[derive(Clone)]
pub struct PgChangeRecorder {
    pool: Pool,
}

impl PgChangeRecorder {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

/// In-transaction handle.
pub struct PgChangeTx<'tx> {
    group_id: GroupId,
    tx: Mutex<sqlx::Transaction<'tx, sqlx::Postgres>>,
}

#[async_trait]
impl<'tx> ChangeTx for PgChangeTx<'tx> {
    fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    async fn record(&self, mut ch: Change) -> Result<ChangeId> {
        let id = new_change_id();
        ch.id = id.clone();
        ch.group_id = self.group_id.clone();
        if ch.at.timestamp_nanos_opt().is_none() {
            ch.at = Utc::now();
        }

        let (actor_kind, actor_id, actor_meta) = actor_columns(&ch.actor)?;
        let resource_id = ch.resource.id.clone().ok_or_else(|| Error::Internal {
            source: "Change::resource.id must be Some for changelog rows".into(),
        })?;
        let op_text = op_to_text(&ch.op);
        let version_i64: Option<i64> = ch.resource_version.map(|v| v as i64);

        let mut guard = self.tx.lock().await;

        sqlx::query(
            r#"
            INSERT INTO starter_changes (
                id, at, actor_kind, actor_id, actor_meta,
                resource_kind, resource_id, resource_owner, resource_version,
                op, before, after, patch, group_id, correlation
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13, $14, $15
            )
            "#,
        )
        .bind(&id.0)
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
impl ChangeRecorder for PgChangeRecorder {
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
        let tx = self.pool.sqlx().begin().await.map_err(internal)?;
        let handle = PgChangeTx {
            group_id: new_group_id(),
            tx: Mutex::new(tx),
        };

        let result = f(&handle).await;
        let PgChangeTx { tx, .. } = handle;
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
        let result = if let Some(id) = &resource.id {
            sqlx::query(
                r#"UPDATE starter_changes
                      SET before = NULL, after = NULL, patch = NULL
                    WHERE resource_kind = $1 AND resource_id = $2"#,
            )
            .bind(&resource.kind)
            .bind(id)
            .execute(self.pool.sqlx())
            .await
        } else {
            sqlx::query(
                r#"UPDATE starter_changes
                      SET before = NULL, after = NULL, patch = NULL
                    WHERE resource_kind = $1"#,
            )
            .bind(&resource.kind)
            .execute(self.pool.sqlx())
            .await
        };
        Ok(result.map_err(internal)?.rows_affected())
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
