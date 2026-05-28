//! Postgres-backed [`UndoCursor`] with epoch-based concurrency.
//!
//! Persists the per-actor redo stack to `starter_undo_cursors`. Two
//! processes racing redo for the same actor cannot both pop the
//! same group: every write is a CAS update keyed on
//! `(actor_key, epoch)`, and the loser observes the bumped epoch
//! and retries.
//!
//! Wire-up:
//!
//! ```ignore
//! use starter_store_postgres::migrate;
//! use starter_undo::{cursor_postgres::{migration_source, PgUndoCursor}, UndoService};
//!
//! migrate(&pool).with_source(migration_source()).run().await?;
//! let cursor = std::sync::Arc::new(PgUndoCursor::new(pool.clone()));
//! let service = UndoService::with_cursor(log, registry, cursor);
//! ```
//!
//! See [`crate::cursor_postgres`] migration SQL for the schema and
//! `docs/design/undo/` for the broader contract.

use async_trait::async_trait;
use serde_json::Value;
use starter_spi::changelog::{Actor, GroupId};
use starter_spi::{Error, Result};
use starter_store_postgres::{MigrationSource, Pool};

use crate::service::{actor_key, UndoCursor};

/// sqlx migrator for the `starter_undo_cursors` table.
pub static UNDO_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/postgres");

/// Migration source identifier. Lives under its own
/// `_sqlx_migrations_undo` table so version numbers never collide
/// with other starter sources.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "undo",
        migrator: &UNDO_MIGRATOR,
    }
}

/// Maximum CAS retries per public op before surfacing a conflict.
///
/// Three is sufficient for any realistic contention level: the only
/// writer that can bump our epoch is *another* process operating on
/// the same actor at the same instant, which under normal UX is
/// rare. Surfacing rather than spinning silently is the right
/// default — the caller knows their stack changed and can re-fetch.
const MAX_CAS_RETRIES: usize = 3;

/// Postgres-backed [`UndoCursor`]. One row per actor; the redo
/// stack lives in a `jsonb` array with the top of the stack at the
/// tail. Writes are CAS updates against `epoch`.
#[derive(Clone)]
pub struct PgUndoCursor {
    pool: Pool,
}

impl PgUndoCursor {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Fetch the stack + epoch for `key`. Returns `(stack, epoch)`;
    /// when no row exists, returns an empty stack at epoch `0` so
    /// the first write can `INSERT … ON CONFLICT` without an extra
    /// round trip.
    async fn fetch(&self, key: &str) -> Result<(Vec<String>, i64)> {
        let row: Option<(Value, i64)> = sqlx::query_as(
            r#"SELECT redo_stack, epoch
                 FROM starter_undo_cursors
                WHERE actor_key = $1"#,
        )
        .bind(key)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(internal)?;

        match row {
            Some((stack, epoch)) => Ok((parse_stack(stack)?, epoch)),
            None => Ok((Vec::new(), 0)),
        }
    }

    /// Persist `stack` at `epoch + 1`, conditional on `epoch`
    /// being unchanged. Returns `true` when the CAS landed, `false`
    /// when another writer raced us.
    async fn cas_write(
        &self,
        key: &str,
        observed_epoch: i64,
        new_stack: &[String],
    ) -> Result<bool> {
        let json = serde_json::to_value(new_stack).map_err(internal_serde)?;

        // Two cases collapse into one statement:
        //   - row exists at `observed_epoch` → UPDATE bumps it.
        //   - row does not exist (observed_epoch must be 0) → INSERT
        //     fresh row.
        // We split them so the predicate is explicit and the
        // optimizer never has to do an EXCLUDED dance.
        if observed_epoch == 0 {
            // First write for this actor — INSERT, fall through to
            // UPDATE on conflict only when the existing row is still
            // at epoch 0 (i.e. nobody else inserted yet).
            let result = sqlx::query(
                r#"INSERT INTO starter_undo_cursors
                       (actor_key, redo_stack, epoch, updated_at)
                       VALUES ($1, $2, 1, NOW())
                       ON CONFLICT (actor_key) DO UPDATE
                       SET redo_stack = EXCLUDED.redo_stack,
                           epoch      = starter_undo_cursors.epoch + 1,
                           updated_at = NOW()
                       WHERE starter_undo_cursors.epoch = 0"#,
            )
            .bind(key)
            .bind(&json)
            .execute(self.pool.sqlx())
            .await
            .map_err(internal)?;
            Ok(result.rows_affected() == 1)
        } else {
            let result = sqlx::query(
                r#"UPDATE starter_undo_cursors
                      SET redo_stack = $2,
                          epoch      = epoch + 1,
                          updated_at = NOW()
                    WHERE actor_key = $1
                      AND epoch     = $3"#,
            )
            .bind(key)
            .bind(&json)
            .bind(observed_epoch)
            .execute(self.pool.sqlx())
            .await
            .map_err(internal)?;
            Ok(result.rows_affected() == 1)
        }
    }
}

#[async_trait]
impl UndoCursor for PgUndoCursor {
    async fn peek_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let key = actor_key(actor);
        let (stack, _) = self.fetch(&key).await?;
        Ok(stack.last().cloned().map(GroupId))
    }

    async fn push_redo(&self, actor: &Actor, group: GroupId) -> Result<()> {
        let key = actor_key(actor);
        for _ in 0..MAX_CAS_RETRIES {
            let (mut stack, epoch) = self.fetch(&key).await?;
            stack.push(group.0.clone());
            if self.cas_write(&key, epoch, &stack).await? {
                return Ok(());
            }
        }
        Err(Error::Conflict {
            message: format!(
                "starter_undo_cursors: CAS push failed after {MAX_CAS_RETRIES} retries"
            ),
        })
    }

    async fn pop_redo(&self, actor: &Actor) -> Result<Option<GroupId>> {
        let key = actor_key(actor);
        for _ in 0..MAX_CAS_RETRIES {
            let (mut stack, epoch) = self.fetch(&key).await?;
            let popped = stack.pop();
            if popped.is_none() {
                // Nothing to pop and nothing to write — return
                // without bumping the epoch.
                return Ok(None);
            }
            if self.cas_write(&key, epoch, &stack).await? {
                return Ok(popped.map(GroupId));
            }
        }
        Err(Error::Conflict {
            message: format!(
                "starter_undo_cursors: CAS pop failed after {MAX_CAS_RETRIES} retries"
            ),
        })
    }

    async fn clear_redo(&self, actor: &Actor) -> Result<()> {
        // A blind DELETE is correct here: clear is idempotent and
        // semantics-equivalent regardless of which writer landed
        // last. The epoch bump on the next push/pop will still
        // expose any concurrent stale reader.
        let key = actor_key(actor);
        sqlx::query(r#"DELETE FROM starter_undo_cursors WHERE actor_key = $1"#)
            .bind(&key)
            .execute(self.pool.sqlx())
            .await
            .map_err(internal)?;
        Ok(())
    }
}

fn parse_stack(raw: Value) -> Result<Vec<String>> {
    serde_json::from_value(raw).map_err(|e| Error::Internal {
        source: Box::new(e),
    })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

fn internal_serde(e: serde_json::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
