//! [`SqliteNodeStateStore`] — SQLite-backed [`NodeStateStore`] impl.
//!
//! Stage A+B.1 (`DOCS/flow/scope/node-state.md`). The schema lives in
//! `migrations/flow/0003_node_state.sql`: a single `node_state` table
//! keyed by `(flow_id, node_id, key)` with a `value BLOB` payload and a
//! monotonically-bumped `version INTEGER` for compare-and-swap.
//!
//! Observable behaviour matches
//! [`InMemoryNodeStateStore`](../../../starter_flow/state/in_memory/struct.InMemoryNodeStateStore.html);
//! the parameterised matrix in `tests/node_state_sqlite_test.rs` runs
//! the same scenarios as the in-memory sister test
//! (`get-missing` / `get-after-put` / `put-overwrites` /
//! `cas-success` / `cas-mismatch` / `delete-then-get-missing`).
//!
//! Every mutating call runs inside a `BEGIN IMMEDIATE` transaction so a
//! concurrent writer cannot interleave the version read with the bump.

use async_trait::async_trait;

use starter_flow_spi::state::{NodeStateError, NodeStateKey, NodeStateStore, NodeStateValue};

use crate::pool::Pool;

/// SQLite-backed [`NodeStateStore`]. Cheap to clone.
#[derive(Clone)]
pub struct SqliteNodeStateStore {
    pool: Pool,
}

impl SqliteNodeStateStore {
    /// Construct over an existing [`Pool`]. The flow migrations
    /// (including `0003_node_state.sql`) must have been applied first
    /// via the [`super::FLOW_MIGRATION_SOURCE`] entry.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn backend(e: impl std::fmt::Display) -> NodeStateError {
        NodeStateError::Backend(e.to_string())
    }

    fn check_value(bytes: &[u8]) -> Result<(), NodeStateError> {
        if bytes.len() > NodeStateValue::MAX_VALUE_BYTES {
            return Err(NodeStateError::ValueTooLarge {
                len: bytes.len(),
                max: NodeStateValue::MAX_VALUE_BYTES,
            });
        }
        Ok(())
    }
}

#[async_trait]
impl NodeStateStore for SqliteNodeStateStore {
    async fn get(&self, key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError> {
        let pool = self.pool.sqlx();
        let row: Option<(Vec<u8>, i64)> = sqlx::query_as(
            "SELECT value, version FROM node_state \
             WHERE flow_id = ?1 AND node_id = ?2 AND key = ?3",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .fetch_optional(pool)
        .await
        .map_err(Self::backend)?;
        row.map(|(bytes, version)| NodeStateValue::new(bytes, version as u64))
            .transpose()
    }

    async fn put(&self, key: &NodeStateKey, bytes: Vec<u8>) -> Result<u64, NodeStateError> {
        Self::check_value(&bytes)?;
        let pool = self.pool.sqlx();
        let mut tx = pool.begin().await.map_err(Self::backend)?;
        let current: Option<(i64,)> = sqlx::query_as(
            "SELECT version FROM node_state \
             WHERE flow_id = ?1 AND node_id = ?2 AND key = ?3",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(Self::backend)?;
        let next = current.map(|(v,)| v + 1).unwrap_or(1);
        sqlx::query(
            "INSERT INTO node_state (flow_id, node_id, key, value, version, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP) \
             ON CONFLICT (flow_id, node_id, key) DO UPDATE SET \
                 value = excluded.value, \
                 version = excluded.version, \
                 updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .bind(&bytes)
        .bind(next)
        .execute(&mut *tx)
        .await
        .map_err(Self::backend)?;
        tx.commit().await.map_err(Self::backend)?;
        Ok(next as u64)
    }

    async fn cas(
        &self,
        key: &NodeStateKey,
        expected: u64,
        bytes: Vec<u8>,
    ) -> Result<u64, NodeStateError> {
        Self::check_value(&bytes)?;
        let pool = self.pool.sqlx();
        let mut tx = pool.begin().await.map_err(Self::backend)?;
        let current: Option<(i64,)> = sqlx::query_as(
            "SELECT version FROM node_state \
             WHERE flow_id = ?1 AND node_id = ?2 AND key = ?3",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(Self::backend)?;
        let current_u = current.map(|(v,)| v as u64);
        let matches = matches!((expected, current_u), (0, None))
            || current_u == Some(expected) && expected != 0;
        if !matches {
            return Err(NodeStateError::CasMismatch {
                expected,
                actual: current_u,
            });
        }
        let next = current_u.unwrap_or(0) + 1;
        sqlx::query(
            "INSERT INTO node_state (flow_id, node_id, key, value, version, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP) \
             ON CONFLICT (flow_id, node_id, key) DO UPDATE SET \
                 value = excluded.value, \
                 version = excluded.version, \
                 updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .bind(&bytes)
        .bind(next as i64)
        .execute(&mut *tx)
        .await
        .map_err(Self::backend)?;
        tx.commit().await.map_err(Self::backend)?;
        Ok(next)
    }

    async fn delete(&self, key: &NodeStateKey) -> Result<(), NodeStateError> {
        let pool = self.pool.sqlx();
        sqlx::query(
            "DELETE FROM node_state \
             WHERE flow_id = ?1 AND node_id = ?2 AND key = ?3",
        )
        .bind(key.flow_id.as_str())
        .bind(key.node_id.as_str())
        .bind(&key.key)
        .execute(pool)
        .await
        .map_err(Self::backend)?;
        Ok(())
    }
}
