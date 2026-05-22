//! [`ChangeRecorder`] / [`ChangeTx`] impls.
//!
//! `transaction` opens an immediate sqlx transaction, mints a single
//! `group_id` *before* the closure runs, hands the closure a
//! [`SqliteChangeTx`] handle whose `record()` calls write rows under
//! that group, and commits when the closure returns `Ok`.

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::changelog::{Change, ChangeId, ChangeRecorder, ChangeTx, GroupId};
use starter_spi::{Error, Result};
use starter_store_sqlite::Pool;
use tokio::sync::Mutex;

use crate::codec::{actor_columns, json_to_text, op_to_text};
use crate::ids::{new_change_id, new_group_id};

/// SQLite-backed [`ChangeRecorder`].
#[derive(Clone)]
pub struct SqliteChangeRecorder {
    pool: Pool,
}

impl SqliteChangeRecorder {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

/// In-transaction handle passed to the closure given to
/// [`SqliteChangeRecorder::transaction`].
pub struct SqliteChangeTx<'tx> {
    group_id: GroupId,
    tx: Mutex<sqlx::Transaction<'tx, sqlx::Sqlite>>,
}

#[async_trait]
impl<'tx> ChangeTx for SqliteChangeTx<'tx> {
    fn group_id(&self) -> &GroupId {
        &self.group_id
    }

    async fn record(&self, mut ch: Change) -> Result<ChangeId> {
        // Recorder owns id + group_id assignment — anything the
        // caller put there is overwritten so the contract holds.
        let id = new_change_id();
        ch.id = id.clone();
        ch.group_id = self.group_id.clone();
        if ch.at.timestamp_nanos_opt().is_none() {
            ch.at = Utc::now();
        }

        let (actor_kind, actor_id, actor_meta, actor_model) = actor_columns(&ch.actor)?;
        let resource_id = ch.resource.id.clone().ok_or_else(|| Error::Internal {
            source: "Change::resource.id must be Some for changelog rows".into(),
        })?;

        let before = json_to_text(&ch.before)?;
        let after = json_to_text(&ch.after)?;
        let patch = json_to_text(&ch.patch)?;
        let op_text = op_to_text(&ch.op);
        let at_text = ch.at.to_rfc3339();
        let version_i64: Option<i64> = ch.resource_version.map(|v| v as i64);

        let mut guard = self.tx.lock().await;

        sqlx::query(
            r#"
            INSERT INTO starter_changes (
                id, at, actor_kind, actor_id, actor_meta, actor_model,
                resource_kind, resource_id, resource_owner, resource_version,
                op, before, after, patch, group_id, correlation
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16
            )
            "#,
        )
        .bind(&id.0)
        .bind(&at_text)
        .bind(&actor_kind)
        .bind(&actor_id)
        .bind(&actor_meta)
        .bind(&actor_model)
        .bind(&ch.resource.kind)
        .bind(&resource_id)
        .bind(&ch.resource.owner)
        .bind(version_i64)
        .bind(&op_text)
        .bind(&before)
        .bind(&after)
        .bind(&patch)
        .bind(&self.group_id.0)
        .bind(ch.correlation.as_ref().map(|t| t.0.clone()))
        .execute(&mut **guard)
        .await
        .map_err(internal)?;

        Ok(id)
    }
}

#[async_trait]
impl ChangeRecorder for SqliteChangeRecorder {
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
        let handle = SqliteChangeTx {
            group_id: new_group_id(),
            tx: Mutex::new(tx),
        };

        let result = f(&handle).await;

        let SqliteChangeTx { tx, .. } = handle;
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
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
