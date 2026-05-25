//! Per-node persistent state — the `NodeStateStore` SPI seam.
//!
//! SCOPE pointer: `DOCS/flow/scope/node-state.md`. The seam exists so a
//! node body can hold a small amount of durable state (the canonical
//! example is the counter node's tick count) without the engine having
//! to grow a generic blob-on-the-side. The trait is a per-key
//! `get`/`put`/`cas`/`delete` surface — *not* a generic KV store; it is
//! scoped to `(flow_id, node_id, key)` so every load-bearing call site
//! routes through the same chokepoint, mirroring R5's
//! "one chokepoint per persistence surface" rule.
//!
//! Two implementations ship in the workspace:
//!
//! - [`InMemoryNodeStateStore`](../../starter_flow/state/in_memory/struct.InMemoryNodeStateStore.html)
//!   in `starter-flow` — `RwLock<HashMap>` behind the same trait, used
//!   by the in-process engine and by every unit test that does not
//!   want to spin up SQLite.
//! - [`SqliteNodeStateStore`](../../starter_store_sqlite/flow/node_state/struct.SqliteNodeStateStore.html)
//!   in `starter-store-sqlite` — `(flow_id, node_id, key)` PK + a
//!   monotonically-bumped `version` column for compare-and-swap.
//!
//! Both impls run the same parameterised test matrix:
//! `get-missing` / `get-after-put` / `put-overwrites` / `cas-success` /
//! `cas-mismatch` / `delete-then-get-missing` (see
//! `tests/node_state_in_memory_test.rs` and
//! `tests/node_state_sqlite_test.rs`).
//!
//! Size caps (enforced at the trait boundary so both impls behave the
//! same way under load):
//!
//! - `key.key` ≤ [`NodeStateKey::MAX_KEY_BYTES`] (256 bytes)
//! - `value.bytes` ≤ [`NodeStateValue::MAX_VALUE_BYTES`] (64 KiB)
//!
//! See `node-state.md` for the `reset_on_redeploy` semantics — the
//! store does not interpret revisions; it is the engine's job to
//! `delete` the key when a flow revision changes shape and the kind
//! opted into reset-on-redeploy.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::flow::FlowId;
use crate::node::NodeId;

/// Fully-qualified address of one persisted state value.
///
/// The three-tuple `(flow_id, node_id, key)` is the primary key in
/// every storage backend. `key` is an opaque user-defined string
/// (typically a single token like `"count"`); it is not a reverse-DNS
/// identifier because the namespace is already scoped by `node_id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NodeStateKey {
    /// Owning flow.
    pub flow_id: FlowId,
    /// Owning node within the flow.
    pub node_id: NodeId,
    /// Per-node key. Caller-chosen; opaque to the store.
    pub key: String,
}

impl NodeStateKey {
    /// Maximum byte length of the `key` field, enforced at the trait
    /// boundary so an oversized key fails the same way in every impl.
    pub const MAX_KEY_BYTES: usize = 256;

    /// Build a key, validating the size cap.
    pub fn new(
        flow_id: FlowId,
        node_id: NodeId,
        key: impl Into<String>,
    ) -> Result<Self, NodeStateError> {
        let key = key.into();
        if key.len() > Self::MAX_KEY_BYTES {
            return Err(NodeStateError::KeyTooLarge {
                len: key.len(),
                max: Self::MAX_KEY_BYTES,
            });
        }
        Ok(Self {
            flow_id,
            node_id,
            key,
        })
    }
}

/// The bytes a node has persisted under a [`NodeStateKey`], together
/// with the monotonically-bumped `version` the store assigns.
///
/// Version starts at `1` on the first successful `put` and increments
/// by one on every subsequent overwrite or successful `cas`. It is
/// opaque to the caller except as the `expected` argument to
/// [`NodeStateStore::cas`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeStateValue {
    /// The persisted bytes.
    pub bytes: Vec<u8>,
    /// Monotonic per-key version assigned by the store.
    pub version: u64,
}

impl NodeStateValue {
    /// Maximum byte length of `bytes`, enforced at the trait boundary
    /// so an oversized value fails the same way in every impl.
    pub const MAX_VALUE_BYTES: usize = 64 * 1024;

    /// Construct a value carrying `bytes` at `version`. Returns
    /// [`NodeStateError::ValueTooLarge`] if `bytes` exceeds the cap.
    pub fn new(bytes: Vec<u8>, version: u64) -> Result<Self, NodeStateError> {
        if bytes.len() > Self::MAX_VALUE_BYTES {
            return Err(NodeStateError::ValueTooLarge {
                len: bytes.len(),
                max: Self::MAX_VALUE_BYTES,
            });
        }
        Ok(Self { bytes, version })
    }
}

/// Error surface returned by every [`NodeStateStore`] method.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NodeStateError {
    /// `key.key` exceeded [`NodeStateKey::MAX_KEY_BYTES`].
    #[error("node-state key too large: {len} > {max} bytes")]
    KeyTooLarge {
        /// Actual byte length supplied.
        len: usize,
        /// Cap (currently 256 bytes).
        max: usize,
    },
    /// `value.bytes` exceeded [`NodeStateValue::MAX_VALUE_BYTES`].
    #[error("node-state value too large: {len} > {max} bytes")]
    ValueTooLarge {
        /// Actual byte length supplied.
        len: usize,
        /// Cap (currently 64 KiB).
        max: usize,
    },
    /// `cas` was called with an `expected` version that did not match
    /// the current stored version. `actual` is the version the store
    /// has on disk; `None` if the row is currently absent.
    #[error("node-state CAS mismatch: expected version {expected}, found {actual:?}")]
    CasMismatch {
        /// Version the caller expected to overwrite.
        expected: u64,
        /// Version currently in the store (or `None` if the row is
        /// absent).
        actual: Option<u64>,
    },
    /// Underlying storage failed (sqlx error, poisoned lock, …). The
    /// message is operator-actionable; it is not part of the API
    /// contract.
    #[error("node-state backend error: {0}")]
    Backend(String),
}

/// Persistent per-node key/value store with optimistic CAS.
///
/// SCOPE pointer: `DOCS/flow/scope/node-state.md`. The trait carries
/// no per-flow scope or per-revision scope beyond what's encoded in
/// [`NodeStateKey`] — the engine is responsible for choosing whether
/// to delete state on revision change (`reset_on_redeploy`); the store
/// just stores.
///
/// Implementations are expected to be cheap to clone (e.g. wrap an
/// `Arc<Pool>` or an `Arc<RwLock<HashMap>>`).
#[async_trait]
pub trait NodeStateStore: Send + Sync {
    /// Read the current value. Returns `Ok(None)` if no row exists.
    async fn get(&self, key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError>;

    /// Unconditionally write `bytes`, bumping the version. Returns the
    /// new version. The first write of a key returns version `1`.
    async fn put(&self, key: &NodeStateKey, bytes: Vec<u8>) -> Result<u64, NodeStateError>;

    /// Compare-and-swap. `expected` must match the version currently on
    /// disk (use `0` to mean "no row exists yet"). On success returns
    /// the new version (one greater than `expected`). On mismatch
    /// returns [`NodeStateError::CasMismatch`] without mutating state.
    async fn cas(
        &self,
        key: &NodeStateKey,
        expected: u64,
        bytes: Vec<u8>,
    ) -> Result<u64, NodeStateError>;

    /// Delete the row if present. Deleting an absent row is a no-op
    /// (returns `Ok(())`).
    async fn delete(&self, key: &NodeStateKey) -> Result<(), NodeStateError>;
}

/// A [`NodeStateStore`] that errors on every mutating call and returns
/// `Ok(None)` from `get`. Used as the default `NodeCtx.state` when an
/// engine is constructed without a real store wired in — most notably
/// in unit tests of node bodies that do not exercise state themselves.
///
/// Picking "errors on write" rather than "silently swallows" preserves
/// the SCOPE R5 chokepoint posture: if a node body unexpectedly tries
/// to persist state in a test that didn't opt into a real store, the
/// test fails loudly rather than masking the missing wiring.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopNodeStateStore;

/// Static reference suitable as the `state` argument to
/// [`crate::node::NodeCtx::new`] in tests and bare-engine
/// configurations.
pub static NOOP_NODE_STATE_STORE: NoopNodeStateStore = NoopNodeStateStore;

#[async_trait]
impl NodeStateStore for NoopNodeStateStore {
    async fn get(&self, _key: &NodeStateKey) -> Result<Option<NodeStateValue>, NodeStateError> {
        Ok(None)
    }

    async fn put(&self, _key: &NodeStateKey, _bytes: Vec<u8>) -> Result<u64, NodeStateError> {
        Err(NodeStateError::Backend(
            "NoopNodeStateStore::put called — wire a real NodeStateStore into NodeCtx".to_owned(),
        ))
    }

    async fn cas(
        &self,
        _key: &NodeStateKey,
        _expected: u64,
        _bytes: Vec<u8>,
    ) -> Result<u64, NodeStateError> {
        Err(NodeStateError::Backend(
            "NoopNodeStateStore::cas called — wire a real NodeStateStore into NodeCtx".to_owned(),
        ))
    }

    async fn delete(&self, _key: &NodeStateKey) -> Result<(), NodeStateError> {
        Err(NodeStateError::Backend(
            "NoopNodeStateStore::delete called — wire a real NodeStateStore into NodeCtx"
                .to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> NodeStateKey {
        NodeStateKey::new(
            FlowId::new("acme.flows.demo").unwrap(),
            NodeId::new("acme.nodes.counter").unwrap(),
            "count",
        )
        .unwrap()
    }

    #[test]
    fn key_too_large_rejected() {
        let big = "x".repeat(NodeStateKey::MAX_KEY_BYTES + 1);
        let err = NodeStateKey::new(
            FlowId::new("acme.flows.demo").unwrap(),
            NodeId::new("acme.nodes.counter").unwrap(),
            big,
        )
        .unwrap_err();
        matches!(err, NodeStateError::KeyTooLarge { .. });
    }

    #[test]
    fn value_too_large_rejected() {
        let big = vec![0u8; NodeStateValue::MAX_VALUE_BYTES + 1];
        let err = NodeStateValue::new(big, 1).unwrap_err();
        matches!(err, NodeStateError::ValueTooLarge { .. });
    }

    #[test]
    fn noop_store_constructs() {
        // Exercise that `NOOP_NODE_STATE_STORE` is reachable as a
        // `&'static dyn NodeStateStore`. Async behaviour is covered
        // by the in-memory + sqlite matrix tests in their owning
        // crates (which have tokio in dev-deps).
        let _k = key();
        let _r: &dyn NodeStateStore = &NOOP_NODE_STATE_STORE;
    }
}
