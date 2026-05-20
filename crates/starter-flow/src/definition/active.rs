//! `ActiveTopology` registry per `DOCS/flow/scope/hot-reload.md` HR2 +
//! HR4.
//!
//! The engine holds one `ArcSwap<FlowTopology>` per live flow. The
//! publish path swaps the inner pointer atomically; in-flight reads
//! get the old [`FlowTopology`] until they drop it, new reads get
//! the new one. No locks. This is the per-flow primitive that makes
//! HR2's "structural swap" both safe and lock-free.
//!
//! This module exposes:
//!
//! - [`ActiveTopology`] \u2014 newtype around `Arc<ArcSwap<FlowTopology>>`
//!   so the engine's accessor signature stays stable as we layer in
//!   per-tenant / per-revision metadata later.
//! - [`ActiveTopologies`] \u2014 `RwLock<HashMap<FlowId, ActiveTopology>>`
//!   per the SCOPE D1b decision pattern. Exposed via
//!   [`Self::get`] / [`Self::install`] / [`Self::remove`]; callers
//!   never touch the inner map.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use tokio::sync::RwLock;

use starter_flow_spi::flow::FlowId;

use crate::propagator::FlowTopology;

/// A live flow's active [`FlowTopology`], swappable atomically via
/// [`Self::store`].
///
/// `Clone` is cheap (`Arc` bump). The publish path holds one clone
/// to call `store`; the engine's run path holds another to call
/// `load` per slot read.
#[derive(Clone)]
pub struct ActiveTopology {
    inner: Arc<ArcSwap<FlowTopology>>,
}

impl ActiveTopology {
    /// Mount a freshly-resolved topology.
    pub fn new(topology: Arc<FlowTopology>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from(topology)),
        }
    }

    /// Atomically replace the active topology. In-flight readers
    /// keep their `Arc` to the previous topology until they drop
    /// it; subsequent [`Self::load`] calls see the new one.
    pub fn store(&self, topology: Arc<FlowTopology>) {
        self.inner.store(topology);
    }

    /// Cheap snapshot of the current topology. The returned
    /// `Arc<FlowTopology>` is safe to hold across awaits \u2014 it
    /// will not be invalidated by a concurrent `store`.
    pub fn load(&self) -> Arc<FlowTopology> {
        self.inner.load_full()
    }
}

/// Per-engine map from [`FlowId`] to live [`ActiveTopology`].
///
/// Mutations (`install`, `remove`) take the write lock briefly;
/// reads (`get`) take the read lock and clone an `Arc` out, then
/// drop the lock immediately \u2014 the actual topology access is
/// lock-free via the `ArcSwap` inside.
#[derive(Default)]
pub struct ActiveTopologies {
    inner: RwLock<HashMap<FlowId, ActiveTopology>>,
}

impl ActiveTopologies {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a live topology handle. `None` if the flow is not
    /// mounted (boot resume hasn't run yet, or the flow was
    /// removed).
    pub async fn get(&self, flow_id: &FlowId) -> Option<ActiveTopology> {
        let guard = self.inner.read().await;
        guard.get(flow_id).cloned()
    }

    /// Mount or update the [`ActiveTopology`] for a flow.
    ///
    /// If the flow is already mounted, swap the inner pointer
    /// atomically via [`ActiveTopology::store`] and return the
    /// existing handle (callers receive the same `Arc` so any
    /// previous `get` clone stays valid). If it's a fresh mount,
    /// install a new entry and return that.
    pub async fn install(
        &self,
        flow_id: FlowId,
        topology: Arc<FlowTopology>,
    ) -> ActiveTopology {
        let mut guard = self.inner.write().await;
        if let Some(existing) = guard.get(&flow_id) {
            existing.store(topology);
            existing.clone()
        } else {
            let active = ActiveTopology::new(topology);
            guard.insert(flow_id, active.clone());
            active
        }
    }

    /// Remove a flow's [`ActiveTopology`].
    ///
    /// Returns the handle (if any) so the caller can keep emitting
    /// observability or drain in-flight runs against the last-known
    /// good topology before dropping it.
    pub async fn remove(&self, flow_id: &FlowId) -> Option<ActiveTopology> {
        let mut guard = self.inner.write().await;
        guard.remove(flow_id)
    }

    /// Number of mounted flows. Read-side; takes the read lock.
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Whether the registry is empty. Read-side; takes the read lock.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};

    fn fid(s: &str) -> FlowId {
        FlowId::new(s).unwrap()
    }

    fn empty_topology() -> Arc<FlowTopology> {
        Arc::new(FlowTopology {
            links: HashMap::new(),
            triggers: BTreeMap::new(),
            behaviors: BTreeMap::new(),
        })
    }

    #[tokio::test]
    async fn install_then_get_returns_handle() {
        let registry = ActiveTopologies::new();
        let _h = registry
            .install(fid("examples.test.a"), empty_topology())
            .await;
        assert!(registry.get(&fid("examples.test.a")).await.is_some());
        assert!(registry.get(&fid("examples.test.b")).await.is_none());
    }

    #[tokio::test]
    async fn second_install_swaps_in_place() {
        let registry = ActiveTopologies::new();
        let h1 = registry
            .install(fid("examples.test.a"), empty_topology())
            .await;
        let top1 = h1.load();

        let new_topo = empty_topology();
        let h2 = registry
            .install(fid("examples.test.a"), new_topo.clone())
            .await;
        // h1 and h2 share the same ArcSwap \u2014 swap from either
        // surface is visible from the other.
        assert!(Arc::ptr_eq(&h1.load(), &new_topo));
        assert!(Arc::ptr_eq(&h2.load(), &new_topo));
        // The previously-held snapshot is still alive (no UAF) but
        // is distinct from the new one.
        assert!(!Arc::ptr_eq(&top1, &new_topo));
    }

    #[tokio::test]
    async fn remove_drops_the_entry() {
        let registry = ActiveTopologies::new();
        registry
            .install(fid("examples.test.a"), empty_topology())
            .await;
        let removed = registry.remove(&fid("examples.test.a")).await;
        assert!(removed.is_some());
        assert!(registry.get(&fid("examples.test.a")).await.is_none());
    }
}
