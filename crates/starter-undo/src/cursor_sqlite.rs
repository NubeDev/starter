//! SQLite-backed [`UndoCursor`].
//!
//! Persists the per-actor redo stack to a starter-owned
//! `starter_undo_cursors` table so undo survives process restarts
//! and works across server instances.
//!
//! Wire-up:
//!
//! ```ignore
//! use starter_store_sqlite::migrate;
//! use starter_undo::{cursor_sqlite::{migration_source, SqliteUndoCursor}, UndoService};
//!
//! migrate(&pool).with_source(migration_source()).run().await?;
//! let cursor = std::sync::Arc::new(SqliteUndoCursor::new(pool.clone()));
//! let service = UndoService::with_cursor(log, registry, cursor);
//! ```

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::changelog::{Actor, GroupId};
use starter_spi::{Error, Result};
use starter_store_sqlite::{MigrationSource, Pool};

use crate::service::{actor_key, UndoCursor};

/// SQLite migrator for the `starter_undo_cursors` table.
pub static UNDO_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/sqlite");

/// Migration source identifier. Lives in its own
/// `_sqlx_migrations_undo` table so version numbers never collide
/// with other starter sources.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "undo",
        migrator: &UNDO_MIGRATOR,
    }
}

/// SQLite-backed [`UndoCursor`]. One row per stack entry under a
/// dense, monotonically increasing `position` per actor.
#[derive(Clone)]
pub struct SqliteUndoCursor {
    pool: Pool,
}

impl SqliteUndoCursor {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UndoCursor for SqliteUndoCursor {
    async fn peek_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let key = actor_key(actor);
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT group_id
                 FROM starter_undo_cursors
                WHERE actor_key = ?1
                ORDER BY position DESC
                LIMIT 1"#,
        )
        .bind(&key)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(internal)?;
        Ok(row.map(|(g,)| GroupId(g)))
    }

    async fn push_redo(&self, actor: &Actor, group: GroupId) -> Result<()> {
        let key = actor_key(actor);
        let pushed_at = Utc::now().to_rfc3339();

        // Single statement so a concurrent pusher on the same actor
        // can't observe a duplicate `position`. SQLite serialises
        // writes, so `COALESCE(MAX(position) + 1, 0)` is safe.
        sqlx::query(
            r#"INSERT INTO starter_undo_cursors (actor_key, position, group_id, pushed_at)
                 VALUES (
                    ?1,
                    COALESCE(
                        (SELECT MAX(position) + 1
                           FROM starter_undo_cursors
                          WHERE actor_key = ?1),
                        0
                    ),
                    ?2,
                    ?3
                 )"#,
        )
        .bind(&key)
        .bind(&group.0)
        .bind(&pushed_at)
        .execute(self.pool.sqlx())
        .await
        .map_err(internal)?;
        Ok(())
    }

    async fn pop_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let key = actor_key(actor);
        let mut tx = self.pool.sqlx().begin().await.map_err(internal)?;

        let row: Option<(i64, String)> = sqlx::query_as(
            r#"SELECT position, group_id
                 FROM starter_undo_cursors
                WHERE actor_key = ?1
                ORDER BY position DESC
                LIMIT 1"#,
        )
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;

        let popped = if let Some((pos, group_id)) = row {
            sqlx::query(
                r#"DELETE FROM starter_undo_cursors
                    WHERE actor_key = ?1 AND position = ?2"#,
            )
            .bind(&key)
            .bind(pos)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
            Some(GroupId(group_id))
        } else {
            None
        };

        tx.commit().await.map_err(internal)?;
        Ok(popped)
    }

    async fn clear_redo(&self, actor: &Actor) -> Result<()> {
        let key = actor_key(actor);
        sqlx::query(r#"DELETE FROM starter_undo_cursors WHERE actor_key = ?1"#)
            .bind(&key)
            .execute(self.pool.sqlx())
            .await
            .map_err(internal)?;
        Ok(())
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
