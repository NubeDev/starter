//! Boot-time construction of the authz [`PolicyEngine`] the tools
//! router runs its permission gate against.
//!
//! Phase 7 — the engine is the DB-backed [`DbPolicyEngine`]
//! loading rules + assignments from `starter_authz_*` tables on
//! every `reload()`. The bootstrap admin allow-all rule is
//! seeded atomically with the schema in
//! `crates/starter-authz/migrations/starter_authz_postgres/0006_bootstrap_admin_rule.sql`
//! so the first request after this swap does NOT lock the
//! operator out (see
//! `rubix/docs/proposal/access-control-redesign.md` §0.3 step 2
//! / §0.4 "Lockout risk").
//!
//! The same registry the StaticRbacEngine used before is fed to
//! `DbPolicyEngine::new` so dashboard-page resource specs stay
//! registered. Returning the concrete `Arc<DbPolicyEngine>`
//! (instead of `Arc<dyn PolicyEngine>`) lets the same handle
//! feed both `middleware::gate_tools` (which only needs the
//! trait) and `AuthzRoutesState` (which needs the concrete type
//! to call `.reload()` after writes).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, ListFilter};
use starter_authz::acl::{summarise, InstanceOwner, RUBIX_DASHBOARD_PAGE};
use starter_authz::audit::DecisionSink;
use starter_authz::instances::{
    InstancesError, InstancesPage, InstancesProvider, InstancesQuery, InstancesRegistry,
    ResourceInstance, SubjectRef,
};
use starter_authz::store::{PolicyStore, PostgresPolicyStore, StoredRule};
use starter_authz::{DbPolicyEngine, StaticRegistry};
use starter_spi::auth::Principal;
use starter_spi::authz::{Ownership, ResourceRegistry, ResourceSpec};
use starter_store_postgres::pool::Pool;

/// Build the rubix-owned resource registry. Kept separate from
/// the engine so consumers needing the registry directly (e.g.
/// `AuthzRoutesState.registry` powering `GET /v1/authz/resources`)
/// can grab it without spinning up the DB-backed engine.
pub fn build_registry() -> Arc<dyn ResourceRegistry> {
    let registry = Arc::new(StaticRegistry::new());
    // The kind + action strings match
    // `crate::middleware::authz_gate::TOOL_RESOURCE_KIND` +
    // `crate::middleware::authz_gate::TOOL_INVOKE_ACTION`.
    registry.register_spec(ResourceSpec::from_static(
        "rubix.tool",
        &["invoke"],
        Ownership::None,
        "Rubix tool",
        "Aggregate resource kind every `rubix.system.*` / `rubix.alert.*` tool dispatch passes through.",
    ));
    // Goal 1, Phase A.1 — SDUI dashboard pages. The
    // `dashboards_seed` boot helper re-registers the same kind
    // on every insert (idempotent via `try_register`).
    registry.register_spec(ResourceSpec::from_static_tenant_scoped(
        "rubix.dashboard.page",
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions` and resolved by the page provider.",
    ));
    registry
}

/// Build the DB-backed engine the tools-router gate consults.
///
/// `default_policy = false` — empty rules table means deny.
/// The bootstrap admin allow-all rule lives in migration `0006`
/// in the `starter-authz` Postgres source so the schema and the
/// unlock-rule are applied as one atomic migration step.
pub async fn build_engine(pool: Pool) -> anyhow::Result<Arc<DbPolicyEngine>> {
    build_engine_with_sink(pool, None).await
}

/// Build the engine and (optionally) install a decision sink
/// before wrapping in `Arc`. `set_sink` takes `&mut self`, so
/// once the engine is `Arc<_>` the sink can no longer be
/// swapped — this is the only seam for wiring `DbDecisionSink`
/// into the audit path during boot.
pub async fn build_engine_with_sink(
    pool: Pool,
    sink: Option<Arc<dyn DecisionSink>>,
) -> anyhow::Result<Arc<DbPolicyEngine>> {
    let registry = build_registry();
    let store = Arc::new(PostgresPolicyStore::new(pool));
    let mut engine = DbPolicyEngine::new(store, registry, false)
        .await
        .map_err(|e| anyhow::anyhow!("build DB authz engine: {e}"))?;
    if let Some(sink) = sink {
        engine
            .set_sink(sink)
            .await
            .map_err(|e| anyhow::anyhow!("install authz decision sink: {e}"))?;
    }
    Ok(Arc::new(engine))
}

/// Default page size when the caller omits `limit`.
const DEFAULT_INSTANCES_LIMIT: u32 = 50;
/// Hard cap on `limit` so a runaway query can't scrape the
/// whole tenant in one round-trip.
const MAX_INSTANCES_LIMIT: u32 = 200;

/// G2 — [`InstancesProvider`] for `rubix.dashboard.page`.
///
/// Lists active dashboard pages for the caller's tenant and
/// derives an [`starter_authz::instances::EffectiveAcl`] per row
/// from the rules table. The v1 mapping is kind-wide (every rule
/// where `resource == "rubix.dashboard.page"` and `tenant_id` is
/// `None` or matches the page's tenant applies to every page);
/// G3 will add a `resource_id` column and bucket per-instance.
pub struct DashboardPageInstancesProvider {
    store: Arc<dyn DashboardStore>,
    policy_store: Arc<dyn PolicyStore>,
}

impl DashboardPageInstancesProvider {
    /// Construct a new provider over the given stores.
    pub fn new(store: Arc<dyn DashboardStore>, policy_store: Arc<dyn PolicyStore>) -> Self {
        Self {
            store,
            policy_store,
        }
    }
}

#[async_trait]
impl InstancesProvider for DashboardPageInstancesProvider {
    async fn list(
        &self,
        _principal: &Principal,
        tenant_id: &str,
        query: InstancesQuery,
    ) -> Result<InstancesPage, InstancesError> {
        let pages = self
            .store
            .list_active(tenant_id, &ListFilter::default())
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;

        // In-memory search filter (case-insensitive title match).
        let search = query.search.as_deref().map(|s| s.to_lowercase());
        let filtered: Vec<_> = pages
            .into_iter()
            .filter(|p| {
                search
                    .as_ref()
                    .is_none_or(|s| p.title.to_lowercase().contains(s))
            })
            .collect();

        // Stable sort: newest-first by created_at, ties broken by
        // page_id so the opaque cursor below round-trips.
        let mut filtered = filtered;
        filtered.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.page_id.cmp(&b.page_id))
        });

        let limit = query
            .limit
            .unwrap_or(DEFAULT_INSTANCES_LIMIT)
            .clamp(1, MAX_INSTANCES_LIMIT) as usize;

        // Opaque cursor: skip everything up to (and including) the
        // entry matching the cursor's `(created_at, page_id)`.
        let start = if let Some(cursor) = query.cursor.as_deref() {
            let decoded = decode_cursor(cursor);
            decoded
                .and_then(|(ts, pid)| {
                    filtered
                        .iter()
                        .position(|p| p.created_at == ts && p.page_id == pid)
                        .map(|i| i + 1)
                })
                .unwrap_or(0)
        } else {
            0
        };

        let rules = self
            .policy_store
            .list_rules()
            .await
            .map_err(|e| InstancesError::Backend(e.to_string()))?;
        // Pre-filter once: kind matches + tenant is global or matches.
        let kind_rules: Vec<&StoredRule> = rules
            .iter()
            .filter(|r| {
                r.resource == RUBIX_DASHBOARD_PAGE
                    && r.tenant_id
                        .as_deref()
                        .is_none_or(|rt| rt == tenant_id)
            })
            .collect();

        let end = (start + limit).min(filtered.len());
        let page_slice = &filtered[start..end];

        let items: Vec<ResourceInstance> = page_slice
            .iter()
            .map(|p| {
                let owner = InstanceOwner {
                    subject: p.owner_principal.clone(),
                };
                let acl = summarise(RUBIX_DASHBOARD_PAGE, &kind_rules, Some(&owner), &p.page_id);
                ResourceInstance {
                    id: p.page_id.clone(),
                    label: p.title.clone(),
                    owner: Some(SubjectRef::User {
                        sub: p.owner_principal.clone(),
                    }),
                    updated_at: Some(p.created_at.clone()),
                    effective_acl: acl,
                }
            })
            .collect();

        let next_cursor = if end < filtered.len() {
            page_slice
                .last()
                .map(|p| encode_cursor(&p.created_at, &p.page_id))
        } else {
            None
        };

        Ok(InstancesPage { items, next_cursor })
    }
}

fn encode_cursor(created_at: &str, page_id: &str) -> String {
    use base64::Engine;
    let raw = format!("{created_at}\u{1f}{page_id}");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes())
}

fn decode_cursor(cursor: &str) -> Option<(String, String)> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let mut parts = s.splitn(2, '\u{1f}');
    let ts = parts.next()?.to_string();
    let pid = parts.next()?.to_string();
    Some((ts, pid))
}

/// Build an [`InstancesRegistry`] with the `rubix.dashboard.page`
/// provider registered. Wired into `AuthzRoutesState` so the
/// `/v1/authz/resources/:kind/instances` route resolves.
pub fn build_instances_registry(
    dashboard_store: Arc<dyn DashboardStore>,
    policy_store: Arc<dyn PolicyStore>,
) -> Arc<InstancesRegistry> {
    let registry = InstancesRegistry::new();
    registry.register(
        RUBIX_DASHBOARD_PAGE,
        Arc::new(DashboardPageInstancesProvider::new(
            dashboard_store,
            policy_store,
        )),
    );
    Arc::new(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubix_spi::dashboard::{
        DashboardRevision, DashboardStore, DashboardStoreError, InsertOutcome, ListFilter,
        NewRevision,
    };
    use starter_authz::instances::ShareScope;
    use starter_authz::store::{PolicyStoreError, StoredAssignment};

    struct FakeDashStore {
        rows: Vec<DashboardRevision>,
    }

    #[async_trait]
    impl DashboardStore for FakeDashStore {
        async fn insert_revision(
            &self,
            _new: NewRevision,
        ) -> Result<DashboardRevision, DashboardStoreError> {
            unimplemented!()
        }
        async fn insert_revision_with_prior(
            &self,
            _new: NewRevision,
        ) -> Result<InsertOutcome, DashboardStoreError> {
            unimplemented!()
        }
        async fn get_active(
            &self,
            _tenant_id: &str,
            _page_id: &str,
        ) -> Result<Option<DashboardRevision>, DashboardStoreError> {
            Ok(None)
        }
        async fn list_active(
            &self,
            tenant_id: &str,
            _filter: &ListFilter,
        ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(self
                .rows
                .iter()
                .filter(|r| r.tenant_id == tenant_id)
                .cloned()
                .collect())
        }
        async fn mark_superseded(
            &self,
            _tenant_id: &str,
            _page_id: &str,
        ) -> Result<u64, DashboardStoreError> {
            Ok(0)
        }
        async fn history(
            &self,
            _page_id: &str,
        ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(vec![])
        }
    }

    struct FakePolicyStore {
        rules: Vec<StoredRule>,
    }

    #[async_trait]
    impl PolicyStore for FakePolicyStore {
        async fn list_assignments(&self) -> Result<Vec<StoredAssignment>, PolicyStoreError> {
            Ok(vec![])
        }
        async fn list_rules(&self) -> Result<Vec<StoredRule>, PolicyStoreError> {
            Ok(self.rules.clone())
        }
        async fn insert_assignment(
            &self,
            _row: &StoredAssignment,
        ) -> Result<(), PolicyStoreError> {
            Ok(())
        }
        async fn delete_assignment(&self, _id: &str) -> Result<(), PolicyStoreError> {
            Ok(())
        }
        async fn insert_rule(&self, _row: &StoredRule) -> Result<(), PolicyStoreError> {
            Ok(())
        }
        async fn update_rule(&self, _row: &StoredRule) -> Result<(), PolicyStoreError> {
            Ok(())
        }
        async fn delete_rule(&self, _id: &str) -> Result<(), PolicyStoreError> {
            Ok(())
        }
    }

    fn rev(page_id: &str, title: &str, owner: &str, created_at: &str) -> DashboardRevision {
        DashboardRevision {
            page_id: page_id.into(),
            revision_id: format!("rev-{page_id}"),
            tenant_id: "t1".into(),
            owner_principal: owner.into(),
            title: title.into(),
            tags: vec![],
            body_json: serde_json::json!({}),
            created_by: owner.into(),
            created_at: created_at.into(),
            superseded_at: None,
        }
    }

    fn rule(role: &str, actions: &[&str], tenant_id: Option<&str>) -> StoredRule {
        StoredRule {
            id: format!("rule-{role}-{}", actions.join(",")),
            role: role.into(),
            resource: RUBIX_DASHBOARD_PAGE.into(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            condition: None,
            effect: "allow".into(),
            priority: 100,
            created_by: "tester".into(),
            tenant_id: tenant_id.map(String::from),
            source: "manual".into(),
            resource_id: None,
        }
    }

    fn principal() -> Principal {
        Principal {
            subject: "alice".into(),
            role: starter_spi::auth::Role::Admin,
            scopes: vec![],
            tenant_id: Some("t1".into()),
            teams: vec![],
            tenant_scope: Vec::new(),
            extra: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn provider_lists_pages_with_summarised_acl() {
        let dash: Arc<dyn DashboardStore> = Arc::new(FakeDashStore {
            rows: vec![
                rev("p1", "Boiler", "alice", "2026-05-01T00:00:00Z"),
                rev("p2", "Chiller", "alice", "2026-05-02T00:00:00Z"),
            ],
        });
        let policy: Arc<dyn PolicyStore> = Arc::new(FakePolicyStore {
            rules: vec![rule("team:hvac-ops", &["view", "edit"], Some("t1"))],
        });
        let registry = build_instances_registry(dash, policy);
        let provider = registry.get(RUBIX_DASHBOARD_PAGE).expect("registered");

        let page = provider
            .list(&principal(), "t1", InstancesQuery::default())
            .await
            .expect("list ok");

        assert_eq!(page.items.len(), 2);
        // Newest-first ordering.
        assert_eq!(page.items[0].id, "p2");
        assert_eq!(page.items[1].id, "p1");
        // Specific share scope because a team holds edit on the kind.
        assert_eq!(page.items[0].effective_acl.share_scope, ShareScope::Specific);
        assert_eq!(page.items[0].effective_acl.grants.len(), 1);
        assert_eq!(page.items[0].updated_at.as_deref(), Some("2026-05-02T00:00:00Z"));
    }

    #[tokio::test]
    async fn search_filters_title_case_insensitively() {
        let dash: Arc<dyn DashboardStore> = Arc::new(FakeDashStore {
            rows: vec![
                rev("p1", "Boiler", "alice", "2026-05-01T00:00:00Z"),
                rev("p2", "Chiller", "alice", "2026-05-02T00:00:00Z"),
            ],
        });
        let policy: Arc<dyn PolicyStore> = Arc::new(FakePolicyStore { rules: vec![] });
        let registry = build_instances_registry(dash, policy);
        let provider = registry.get(RUBIX_DASHBOARD_PAGE).unwrap();

        let q = InstancesQuery {
            search: Some("boil".into()),
            ..Default::default()
        };
        let page = provider.list(&principal(), "t1", q).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "p1");
        assert_eq!(page.items[0].effective_acl.share_scope, ShareScope::Private);
    }
}
