//! In-memory [`NodeStateStore`] backed by `RwLock<HashMap>`.
//!
//! Stage A+B.1 (`DOCS/flow/scope/node-state.md`). Mirrors the public
//! contract of [`starter_store_sqlite::flow::node_state::SqliteNodeStateStore`]
//! exactly; the parameterised matrix in
//! `tests/node_state_in_memory_test.rs` and
//! `tests/node_state_sqlite_test.rs` exercises both impls through the
//! same scenarios (`get-missing` / `get-after-put` / `put-overwrites`
//! / `cas-success` / `cas-mismatch` / `delete-then-get-missing`).
//!
//! Versions are stored alongside the bytes; the store assigns version
//! `1` on the first `put` and increments by one on every overwrite or
//! successful `cas`. A `cas` with `expected = 0` succeeds only when no
//! row exists for the key.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use starter_flow_spi::state::{NodeStateError, NodeStateKey, NodeStateStore, NodeStateValue};

/// In-process [`NodeStateStore`] keyed by `(flow_id, node_id, key)`.
///
/// Cheap to clone — the entire state hides behind a single
/// `Arc<RwLock<HashMap>>`. Pass `Arc<Self>` into the propagator
/// builder (`spawn_with_checkpoint(.., node_state)`) to wire this as
/// the per-engine default.
#[derive(Debug, Default, Clone)]
#[allow(clippy::type_complexity)]
pub struct InMemoryNodeStateStore {
    inner: Arc<RwLock<HashMap<NodeStateKey, (Vec<u8>, u64)>>>,
}

impl InMemoryNodeStateStore {
    /// Construct a fresh, empty store.
    pub fn new() -> Self {
        Self::default()
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
impl NodeStateStore for InMemoryNodeStateStore {
    async fn get(&self, key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError> {
        let guard = self.inner.read().await;
        guard
            .get(key)
            .map(|(b, v)| NodeStateValue::new(b.clone(), *v))
            .transpose()
    }

    async fn put(&self, key: &NodeStateKey, bytes: Vec<u8>) -> Result<u64, NodeStateError> {
        Self::check_value(&bytes)?;
        let mut guard = self.inner.write().await;
        let next = guard.get(key).map(|(_, v)| v + 1).unwrap_or(1);
        guard.insert(key.clone(), (bytes, next));
        Ok(next)
    }

    async fn cas(
        &self,
        key: &NodeStateKey,
        expected: u64,
        bytes: Vec<u8>,
    ) -> Result<u64, NodeStateError> {
        Self::check_value(&bytes)?;
        let mut guard = self.inner.write().await;
        let current = guard.get(key).map(|(_, v)| *v);
        let matches = match (expected, current) {
            (0, None) => true,
            (e, Some(c)) if e == c => true,
            _ => false,
        };
        if !matches {
            return Err(NodeStateError::CasMismatch {
                expected,
                actual: current,
            });
        }
        let next = current.unwrap_or(0) + 1;
        guard.insert(key.clone(), (bytes, next));
        Ok(next)
    }

    async fn delete(&self, key: &NodeStateKey) -> Result<(), NodeStateError> {
        let mut guard = self.inner.write().await;
        guard.remove(key);
        Ok(())
    }
}
