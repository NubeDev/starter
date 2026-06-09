//! `InstancesProvider` for `nexus.dashboard` — the seam that lets the authz
//! admin surface (`GET /v1/authz/resources/nexus.dashboard/instances`) list a
//! tenant's dashboards with their effective ACL, so the UI can render a
//! Grafana-style "who has access" / share view per dashboard.
//!
//! The generic machinery lives in `starter-authz`: this provider only lists the
//! tenant's dashboards (the store owns them) and hands each to `acl::summarise`,
//! which derives `share_scope` + per-subject tiers from the tenant's grant rules.
//! Nexus dashboards have no owner column, so every instance reports `owner: None`
//! and the summary classifies share scope from the grant rules alone.

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use starter_authz::acl::summarise;
use starter_authz::instances::{
    InstancesError, InstancesPage, InstancesProvider, InstancesQuery, ResourceInstance,
};
use starter_authz::store::{PolicyStore, StoredRule};
use starter_spi::auth::Principal;

use crate::authz::KIND_DASHBOARD;

/// Lists `nexus.dashboard` instances + their ACL for the authz admin surface.
pub struct DashboardInstancesProvider {
    /// The control-plane pool; `dashboard::list` runs tenant-scoped under RLS.
    metadata: PgPool,
    /// The grant-rule store, read once per listing to summarise each ACL.
    policy_store: Arc<dyn PolicyStore>,
}

impl DashboardInstancesProvider {
    /// Build the provider over the metadata pool and the shared policy store.
    pub fn new(metadata: PgPool, policy_store: Arc<dyn PolicyStore>) -> Self {
        Self {
            metadata,
            policy_store,
        }
    }
}

#[async_trait]
impl InstancesProvider for DashboardInstancesProvider {
    async fn list(
        &self,
        _principal: &Principal,
        tenant_id: &str,
        query: InstancesQuery,
    ) -> Result<InstancesPage, InstancesError> {
        let dashboards = nexus_store::dashboard::list(&self.metadata, tenant_id)
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;

        // Case-insensitive name filter, matching the provider contract's free-text
        // search. The store already orders newest-first.
        let search = query.search.as_deref().map(str::to_lowercase);
        let matched: Vec<_> = dashboards
            .into_iter()
            .filter(|d| {
                search
                    .as_ref()
                    .is_none_or(|s| d.name.to_lowercase().contains(s))
            })
            .collect();

        // Load the tenant's dashboard-kind rules once, then summarise each
        // instance against them — no per-dashboard rule query (avoids N+1).
        let rules = self
            .policy_store
            .list_rules()
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;
        let kind_rules: Vec<&StoredRule> = rules
            .iter()
            .filter(|r| {
                r.resource == KIND_DASHBOARD
                    && r.tenant_id.as_deref().is_none_or(|rt| rt == tenant_id)
            })
            .collect();

        let items: Vec<ResourceInstance> = matched
            .iter()
            .map(|d| {
                let id = d.id.to_string();
                let acl = summarise(KIND_DASHBOARD, &kind_rules, None, &id);
                ResourceInstance {
                    id,
                    label: d.name.clone(),
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
