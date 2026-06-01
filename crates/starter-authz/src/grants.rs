//! G3 — Grants API. A thin sugar layer over [`PolicyStore`] that
//! writes a single rule row per grant, marked with
//! `source = "grant"`. The engine itself is unchanged: it still
//! evaluates the rules table; the grants surface is just a
//! constrained way of writing into it.
//!
//! Mapping (`rubix.dashboard.page`, v1 kind):
//!
//! | Grant                                          | Rule row                                                                                           |
//! |-----------------------------------------------|----------------------------------------------------------------------------------------------------|
//! | `{ subject: team:hvac-ops, tier: Edit, page x }` | `{ role: "team:hvac-ops", resource: "rubix.dashboard.page", resource_id: "x", actions: ["view","edit"], effect: "allow", source: "grant", priority: 100 }` |
//!
//! The role-resolution side already maps `Principal.teams` to
//! `team:<slug>` role strings (see `engine.rs::roles_for`), so a
//! grant for a team member resolves to Allow without any
//! engine change.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::acl::actions_for_tier;
use crate::instances::{PermissionTier, ShareScope, SubjectRef};
use crate::store::{PolicyStore, PolicyStoreError, StoredRule};

/// Errors from the grants surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GrantError {
    /// Underlying store failure.
    #[error("grant store: {0}")]
    Store(#[from] PolicyStoreError),
    /// Tier mapping is undefined for the given kind. v1 only ships
    /// `rubix.dashboard.page`; other kinds 422 at the HTTP layer.
    #[error("grant kind `{kind}` has no tier mapping")]
    UnsupportedKind {
        /// The resource kind that was requested.
        kind: String,
    },
}

/// Payload for `POST /v1/authz/grants`.
#[derive(Debug, Clone, Deserialize)]
pub struct NewGrant {
    /// Subject of the grant — team / user / wildcard.
    pub subject: GrantSubject,
    /// Resource kind. Only `rubix.dashboard.page` is supported in v1.
    pub resource_kind: String,
    /// Specific instance id; `None` is kind-wide and rare.
    #[serde(default)]
    pub resource_id: Option<String>,
    /// Permission tier — drives the action expansion.
    pub tier: PermissionTier,
    /// Tenant scope. Required: grants are always tenant-bound.
    pub tenant_id: String,
}

/// Server view of a grant — round-trippable to a [`StoredRule`].
#[derive(Debug, Clone, Serialize)]
pub struct Grant {
    /// Primary key of the backing rule row.
    pub id: String,
    /// Subject the grant binds.
    pub subject: GrantSubject,
    /// Resource kind.
    pub resource_kind: String,
    /// Specific instance id, or `None` for kind-wide.
    pub resource_id: Option<String>,
    /// Tier.
    pub tier: PermissionTier,
    /// Tenant.
    pub tenant_id: String,
}

/// Subject of a grant — mirrors [`SubjectRef`] but with a stable
/// JSON shape suitable for request bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum GrantSubject {
    /// `team:<slug>`.
    Team {
        /// Team slug.
        slug: String,
    },
    /// `user:<sub>`.
    User {
        /// Subject id of the user.
        sub: String,
    },
    /// `"*"` — every authenticated tenant member.
    Wildcard,
}

impl GrantSubject {
    /// Render to the canonical `role` column form used by the
    /// engine (`team:<slug>` / `user:<sub>` / `"*"`).
    pub fn to_role(&self) -> String {
        match self {
            Self::Team { slug } => format!("team:{slug}"),
            Self::User { sub } => format!("user:{sub}"),
            Self::Wildcard => "*".to_string(),
        }
    }

    /// Parse from a rule's `role` column when the role matches one
    /// of the canonical grant subject forms.
    pub fn from_role(role: &str) -> Option<Self> {
        SubjectRef::parse(role).map(|s| match s {
            SubjectRef::Team { slug } => Self::Team { slug },
            SubjectRef::User { sub } => Self::User { sub },
            SubjectRef::Wildcard => Self::Wildcard,
        })
    }
}

/// Filter for [`GrantStore::list`].
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GrantFilter {
    /// Filter by subject (matches the rule's `role` column).
    #[serde(default)]
    pub subject: Option<String>,
    /// Filter by resource kind.
    #[serde(default)]
    pub resource_kind: Option<String>,
    /// Filter by resource id.
    #[serde(default)]
    pub resource_id: Option<String>,
    /// Filter by tenant.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// CRUD wrapper around [`PolicyStore`] that enforces the
/// `source = "grant"` marker + tier-expansion contract.
pub struct GrantStore {
    store: Arc<dyn PolicyStore>,
    /// Default priority for grant rows.
    priority: i32,
}

impl GrantStore {
    /// Build a new grant store over the given policy store.
    pub fn new(store: Arc<dyn PolicyStore>) -> Self {
        Self { store, priority: 100 }
    }

    /// Borrow the underlying policy store — used by HTTP handlers
    /// that also need to reload the engine cache.
    pub fn policy_store(&self) -> &Arc<dyn PolicyStore> {
        &self.store
    }

    /// Create a grant. Writes one rule row.
    pub async fn create(&self, new: NewGrant, created_by: &str) -> Result<Grant, GrantError> {
        let actions = actions_for_tier(&new.resource_kind, new.tier)
            .ok_or_else(|| GrantError::UnsupportedKind {
                kind: new.resource_kind.clone(),
            })?;
        let id = uuid::Uuid::new_v4().to_string();
        let row = StoredRule {
            id: id.clone(),
            role: new.subject.to_role(),
            resource: new.resource_kind.clone(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            condition: None,
            effect: "allow".into(),
            priority: self.priority,
            created_by: created_by.to_string(),
            tenant_id: Some(new.tenant_id.clone()),
            source: "grant".to_string(),
            resource_id: new.resource_id.clone(),
        };
        self.store.insert_rule(&row).await?;
        Ok(Grant {
            id,
            subject: new.subject,
            resource_kind: new.resource_kind,
            resource_id: new.resource_id,
            tier: new.tier,
            tenant_id: new.tenant_id,
        })
    }

    /// Delete a grant by id. Returns
    /// [`PolicyStoreError::NotFound`] when the id doesn't exist.
    pub async fn delete(&self, id: &str) -> Result<(), GrantError> {
        self.store.delete_rule(id).await?;
        Ok(())
    }

    /// Patch a grant's tier — rewrites the rule's actions list
    /// in place.
    pub async fn patch_tier(
        &self,
        id: &str,
        tier: PermissionTier,
        created_by: &str,
    ) -> Result<Grant, GrantError> {
        let rules = self.store.list_rules().await?;
        let existing = rules
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| PolicyStoreError::NotFound(format!("rule {id}")))?;
        let actions = actions_for_tier(&existing.resource, tier).ok_or_else(|| {
            GrantError::UnsupportedKind {
                kind: existing.resource.clone(),
            }
        })?;
        let row = StoredRule {
            actions: actions.iter().map(|s| s.to_string()).collect(),
            created_by: created_by.to_string(),
            ..existing
        };
        self.store.update_rule(&row).await?;
        let subject = GrantSubject::from_role(&row.role).unwrap_or(GrantSubject::Wildcard);
        Ok(Grant {
            id: row.id,
            subject,
            resource_kind: row.resource,
            resource_id: row.resource_id,
            tier,
            tenant_id: row.tenant_id.unwrap_or_default(),
        })
    }

    /// List grants matching the filter. Reads the entire rules
    /// table (priority-DESC) and filters down to
    /// `source == "grant"`.
    pub async fn list(&self, filter: GrantFilter) -> Result<Vec<Grant>, GrantError> {
        let rules = self.store.list_rules().await?;
        let mut out = Vec::new();
        for r in rules {
            if r.source != "grant" {
                continue;
            }
            if let Some(s) = &filter.subject {
                if &r.role != s {
                    continue;
                }
            }
            if let Some(k) = &filter.resource_kind {
                if &r.resource != k {
                    continue;
                }
            }
            if let Some(rid) = &filter.resource_id {
                if r.resource_id.as_deref() != Some(rid.as_str()) {
                    continue;
                }
            }
            if let Some(t) = &filter.tenant_id {
                if r.tenant_id.as_deref() != Some(t.as_str()) {
                    continue;
                }
            }
            let Some(subject) = GrantSubject::from_role(&r.role) else {
                continue;
            };
            let Some(tier) = crate::acl::tier_for_actions(&r.resource, &r.actions) else {
                continue;
            };
            out.push(Grant {
                id: r.id,
                subject,
                resource_kind: r.resource,
                resource_id: r.resource_id,
                tier,
                tenant_id: r.tenant_id.unwrap_or_default(),
            });
        }
        Ok(out)
    }

    /// Reconcile rules to match the requested share scope for a
    /// `(kind, resource_id, tenant)` tuple.
    ///
    /// - `Private` — deletes every `source="grant"` row for the
    ///   tuple. The owner can still reach the page via the
    ///   engine's ownership condition on seed rules.
    /// - `Tenant` — deletes every `source="grant"` row, then
    ///   inserts one `{ role: "*", actions: ["view"] }` rule so
    ///   any tenant member can read.
    /// - `Specific` — no-op; the explicit grants list is
    ///   authoritative.
    pub async fn set_share_scope(
        &self,
        kind: &str,
        resource_id: &str,
        tenant_id: &str,
        scope: ShareScope,
        created_by: &str,
    ) -> Result<(), GrantError> {
        let rules = self.store.list_rules().await?;
        let targeted: Vec<String> = rules
            .iter()
            .filter(|r| {
                r.source == "grant"
                    && r.resource == kind
                    && r.resource_id.as_deref() == Some(resource_id)
                    && r.tenant_id.as_deref() == Some(tenant_id)
            })
            .map(|r| r.id.clone())
            .collect();
        match scope {
            ShareScope::Private => {
                for id in targeted {
                    let _ = self.store.delete_rule(&id).await;
                }
            }
            ShareScope::Tenant => {
                for id in targeted {
                    let _ = self.store.delete_rule(&id).await;
                }
                let actions = actions_for_tier(kind, PermissionTier::View).ok_or_else(|| {
                    GrantError::UnsupportedKind {
                        kind: kind.to_string(),
                    }
                })?;
                let row = StoredRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    role: "*".into(),
                    resource: kind.to_string(),
                    actions: actions.iter().map(|s| s.to_string()).collect(),
                    condition: None,
                    effect: "allow".into(),
                    priority: self.priority,
                    created_by: created_by.to_string(),
                    tenant_id: Some(tenant_id.to_string()),
                    source: "grant".to_string(),
                    resource_id: Some(resource_id.to_string()),
                };
                self.store.insert_rule(&row).await?;
            }
            ShareScope::Specific => { /* no-op */ }
        }
        Ok(())
    }
}

// Re-export the canonical page kind so HTTP handlers + tests can
// reference it from this module.
pub use crate::acl::RUBIX_DASHBOARD_PAGE as PAGE_KIND;

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::acl::RUBIX_DASHBOARD_PAGE;
    use crate::store::SqlitePolicyStore;
    use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

    async fn store() -> GrantStore {
        let pool = ephemeral().await;
        migrate(&pool)
            .with_source(MigrationSource {
                name: "starter_authz",
                migrator: &crate::store::AUTHZ_SQLITE_MIGRATOR,
            })
            .run()
            .await
            .expect("migrate");
        GrantStore::new(Arc::new(SqlitePolicyStore::new(pool)))
    }

    fn new_grant(slug: &str, tier: PermissionTier, page: &str) -> NewGrant {
        NewGrant {
            subject: GrantSubject::Team {
                slug: slug.into(),
            },
            resource_kind: RUBIX_DASHBOARD_PAGE.into(),
            resource_id: Some(page.into()),
            tier,
            tenant_id: "t1".into(),
        }
    }

    mod create {
        use super::*;

        #[tokio::test]
        async fn expands_edit_tier_to_view_plus_edit() {
            let gs = store().await;
            let g = gs
                .create(new_grant("hvac-ops", PermissionTier::Edit, "p1"), "admin")
                .await
                .unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            let r = rules.into_iter().find(|r| r.id == g.id).unwrap();
            assert!(r.actions.contains(&"view".to_string()));
            assert!(r.actions.contains(&"edit".to_string()));
            assert!(!r.actions.contains(&"delete".to_string()));
        }

        #[tokio::test]
        async fn writes_source_marker() {
            let gs = store().await;
            let g = gs
                .create(new_grant("hvac-ops", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            let r = rules.into_iter().find(|r| r.id == g.id).unwrap();
            assert_eq!(r.source, "grant");
            assert_eq!(r.priority, 100);
            assert_eq!(r.resource_id.as_deref(), Some("p1"));
        }

        #[tokio::test]
        async fn role_team_slug_format() {
            let gs = store().await;
            let g = gs
                .create(new_grant("hvac-ops", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            let r = rules.into_iter().find(|r| r.id == g.id).unwrap();
            assert_eq!(r.role, "team:hvac-ops");
        }
    }

    mod delete {
        use super::*;

        #[tokio::test]
        async fn removes_only_target_row() {
            let gs = store().await;
            let a = gs
                .create(new_grant("hvac-ops", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            let b = gs
                .create(new_grant("alerts", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            gs.delete(&a.id).await.unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            assert!(rules.iter().all(|r| r.id != a.id));
            assert!(rules.iter().any(|r| r.id == b.id));
        }
    }

    mod patch {
        use super::*;

        #[tokio::test]
        async fn tier_update_rewrites_actions_in_place() {
            let gs = store().await;
            let g = gs
                .create(new_grant("hvac-ops", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            let patched = gs
                .patch_tier(&g.id, PermissionTier::Manage, "admin")
                .await
                .unwrap();
            assert_eq!(patched.tier, PermissionTier::Manage);
            let rules = gs.store.list_rules().await.unwrap();
            let r = rules.into_iter().find(|r| r.id == g.id).unwrap();
            assert!(r.actions.contains(&"delete".to_string()));
            assert_eq!(r.id, g.id, "id stable");
        }
    }

    mod share_scope {
        use super::*;

        #[tokio::test]
        async fn tenant_writes_wildcard_subject_view_rule() {
            let gs = store().await;
            gs.create(new_grant("hvac-ops", PermissionTier::Edit, "p1"), "admin")
                .await
                .unwrap();
            gs.set_share_scope(
                RUBIX_DASHBOARD_PAGE,
                "p1",
                "t1",
                ShareScope::Tenant,
                "admin",
            )
            .await
            .unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            let wild: Vec<_> = rules
                .iter()
                .filter(|r| r.source == "grant" && r.role == "*" && r.resource_id.as_deref() == Some("p1"))
                .collect();
            assert_eq!(wild.len(), 1);
            assert_eq!(wild[0].actions, vec!["view".to_string()]);
            // Prior team-grant was deleted as part of the reconcile.
            assert!(!rules
                .iter()
                .any(|r| r.source == "grant" && r.role == "team:hvac-ops"));
        }

        #[tokio::test]
        async fn private_deletes_all_grant_rows() {
            let gs = store().await;
            gs.create(new_grant("hvac-ops", PermissionTier::Edit, "p1"), "admin")
                .await
                .unwrap();
            gs.create(new_grant("alerts", PermissionTier::View, "p1"), "admin")
                .await
                .unwrap();
            gs.set_share_scope(
                RUBIX_DASHBOARD_PAGE,
                "p1",
                "t1",
                ShareScope::Private,
                "admin",
            )
            .await
            .unwrap();
            let rules = gs.store.list_rules().await.unwrap();
            assert!(rules
                .iter()
                .all(|r| !(r.source == "grant" && r.resource_id.as_deref() == Some("p1"))));
        }
    }
}
