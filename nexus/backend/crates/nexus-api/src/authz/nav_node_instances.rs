//! `InstancesProvider` for `nexus.nav_node` — the seam that lets the Access
//! admin surface (`GET /v1/authz/resources/nexus.nav_node/instances`) list a
//! tenant's nav nodes with their effective ACL, so the UI renders share-scope +
//! grants per node (WS-13 §6). This *replaces* per-dashboard sharing: the node
//! is the unit a user navigates to, so the node is what's granted.
//!
//! Mirrors `dashboard_instances.rs`: list the tenant's nodes (the store owns
//! them) and hand each to `acl::summarise`, which derives `share_scope` +
//! per-subject tiers from the tenant's `nav_node`-kind grant rules. Nav nodes
//! have no owner column, so every instance reports `owner: None`.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use starter_authz::acl::summarise;
use starter_authz::instances::{
    InstancesError, InstancesPage, InstancesProvider, InstancesQuery, ResourceInstance,
};
use starter_authz::store::{PolicyStore, StoredRule};
use starter_spi::auth::Principal;

use crate::authz::KIND_NAV_NODE;

/// Lists `nexus.nav_node` instances + their ACL for the Access admin surface.
pub struct NavNodeInstancesProvider {
    /// The control-plane pool; `nav_node::list` runs tenant-scoped under RLS.
    metadata: PgPool,
    /// The grant-rule store, read once per listing to summarise each ACL.
    policy_store: Arc<dyn PolicyStore>,
}

impl NavNodeInstancesProvider {
    /// Build the provider over the metadata pool and the shared policy store.
    pub fn new(metadata: PgPool, policy_store: Arc<dyn PolicyStore>) -> Self {
        Self {
            metadata,
            policy_store,
        }
    }
}

#[async_trait]
impl InstancesProvider for NavNodeInstancesProvider {
    async fn list(
        &self,
        _principal: &Principal,
        tenant_id: &str,
        query: InstancesQuery,
    ) -> Result<InstancesPage, InstancesError> {
        let nodes = nexus_store::nav_node::list(&self.metadata, tenant_id)
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;

        // Case-insensitive title filter, matching the provider contract's
        // free-text search. The store orders the tree parent-first.
        let search = query.search.as_deref().map(str::to_lowercase);
        let matched: Vec<_> = nodes
            .into_iter()
            .filter(|n| {
                search
                    .as_ref()
                    .is_none_or(|s| n.title.to_lowercase().contains(s))
            })
            .collect();

        // Load the tenant's nav_node-kind rules once, then summarise each
        // instance against them — no per-node rule query (avoids N+1).
        let rules = self
            .policy_store
            .list_rules()
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;
        let kind_rules: Vec<&StoredRule> = rules
            .iter()
            .filter(|r| {
                r.resource == KIND_NAV_NODE
                    && r.tenant_id.as_deref().is_none_or(|rt| rt == tenant_id)
            })
            .collect();

        let items: Vec<ResourceInstance> = matched
            .iter()
            .map(|n| {
                let id = n.id.to_string();
                let acl = summarise(KIND_NAV_NODE, &kind_rules, None, &id);
                ResourceInstance {
                    id,
                    label: n.title.clone(),
                    owner: None,
                    updated_at: None,
                    effective_acl: acl,
                }
            })
            .collect();

        Ok(InstancesPage {
            items,
            next_cursor: None,
        })
    }
}
