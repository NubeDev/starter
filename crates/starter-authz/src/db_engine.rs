//! DB-backed policy engine. Thin facade over [`crate::PolicyStore`]
//! + [`crate::StaticRbacEngine`]: every `reload` snapshot the
//! whole rule/assignment table, recompile, swap the inner engine.
//! `check` always hits the cached `StaticRbacEngine` so the hot
//! path is allocation-free.
//!
//! The SCOPE doc's "Open Questions" leaves room for an LRU keyed
//! on `(subject, role_set, resource, action)`; that's deferred —
//! the current cache strategy is "load all, recompile on writes"
//! because the admin REST routes call [`Self::reload`] right after
//! every mutation. Bounded-size tables, bounded-cardinality
//! evaluation: fine for V1.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use starter_spi::auth::Principal;
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef, ResourceRegistry};

use crate::config::AuthzConfig;
use crate::engine::StaticRbacEngine;
use crate::error::{Error, Result};
use crate::store::{PolicyStore, PolicyStoreError};

/// Policy engine that reads rules + assignments from a
/// [`PolicyStore`]. Wrap any backend (sqlite, postgres, in-memory
/// fake) to plug into the standard [`PolicyEngine`] surface.
pub struct DbPolicyEngine {
    store: Arc<dyn PolicyStore>,
    registry: Arc<dyn ResourceRegistry>,
    /// Built-in `Reader`/`Writer`/`Admin` rules are layered in
    /// when this is true. Mirrors
    /// [`AuthzConfig::default_policy`].
    default_policy: bool,
    inner: RwLock<Arc<StaticRbacEngine>>,
}

impl DbPolicyEngine {
    /// Build the engine and prime its cache by loading rules from
    /// the store. Returns an error if the initial load fails — a
    /// boot-time database with no `starter_authz` migration
    /// applied surfaces as `Error::Config` here, not a panic.
    pub async fn new(
        store: Arc<dyn PolicyStore>,
        registry: Arc<dyn ResourceRegistry>,
        default_policy: bool,
    ) -> Result<Self> {
        let inner = Self::compile(&*store, registry.clone(), default_policy).await?;
        Ok(Self {
            store,
            registry,
            default_policy,
            inner: RwLock::new(Arc::new(inner)),
        })
    }

    /// Re-snapshot the store and swap the cached engine. Called
    /// by the admin REST handlers after every successful write.
    /// Cheap enough to do unconditionally — recompilation is
    /// O(rules) and `check` callers see the new engine on their
    /// very next call (the read-side never blocks because we swap
    /// an `Arc`).
    pub async fn reload(&self) -> Result<()> {
        let fresh = Self::compile(&*self.store, self.registry.clone(), self.default_policy).await?;
        *self.inner.write().expect("authz cache lock poisoned") = Arc::new(fresh);
        Ok(())
    }

    /// Borrow the backing store. Used by the admin REST handlers
    /// to perform CRUD without re-threading state.
    pub fn store(&self) -> &Arc<dyn PolicyStore> {
        &self.store
    }

    async fn compile(
        store: &dyn PolicyStore,
        registry: Arc<dyn ResourceRegistry>,
        default_policy: bool,
    ) -> Result<StaticRbacEngine> {
        let stored_rules = store.list_rules().await.map_err(map_store_err)?;
        let stored_assignments = store.list_assignments().await.map_err(map_store_err)?;

        let mut cfg_rules = Vec::with_capacity(stored_rules.len());
        for r in &stored_rules {
            cfg_rules.push(r.to_config().map_err(map_store_err)?);
        }
        let cfg_assignments = stored_assignments.iter().map(|a| a.to_config()).collect();

        let cfg = AuthzConfig {
            default_policy,
            assignments: cfg_assignments,
            rules: cfg_rules,
        };
        StaticRbacEngine::from_config(cfg, registry)
    }
}

fn map_store_err(e: PolicyStoreError) -> Error {
    // Bubble store errors out as config errors — they all share
    // the same "engine cannot be built" semantics from the
    // caller's POV.
    Error::Config(e.to_string())
}

#[async_trait]
impl PolicyEngine for DbPolicyEngine {
    async fn check(&self, principal: &Principal, action: &str, resource: &ResourceRef) -> Decision {
        // Snapshot the Arc under a read lock so `check` never
        // races with `reload`.
        let inner = self
            .inner
            .read()
            .expect("authz cache lock poisoned")
            .clone();
        inner.check(principal, action, resource).await
    }
}
