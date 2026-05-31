//! Resource-instance provider seam.
//!
//! `GET /v1/authz/resources` enumerates *kinds* (the catalogue
//! a registry exposes). The admin UI's "Pages" tab needs the
//! *instances* of a kind — the actual dashboard pages, tools,
//! whatever — with their effective ACL summary. Those instances
//! don't live in authz tables; they live in whatever crate owns
//! the kind. This module is the seam between authz and those
//! owner crates.
//!
//! A consumer crate (e.g. `rubix-agent`) implements
//! [`InstancesProvider`] for a kind it owns and registers the
//! provider with an [`InstancesRegistry`]. The registry is held
//! alongside the existing `ResourceRegistry` on the routes state;
//! `GET /v1/authz/resources/:kind/instances` looks the provider
//! up by kind and calls [`InstancesProvider::list`].
//!
//! Authz contributes the [`EffectiveAcl`] summariser
//! (`crate::acl`) the provider calls per page to derive
//! `share_scope`, `grants`, and `has_legacy_rules` from the
//! tenant's rules table.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::auth::Principal;

/// Provider that lists instances of a registered resource kind
/// for a tenant. Implementations live in the crate that owns the
/// kind (e.g. `rubix-agent` owns `rubix.dashboard.page`).
#[async_trait]
pub trait InstancesProvider: Send + Sync + 'static {
    /// List instances for the principal's tenant. Returns 404 at
    /// the HTTP layer when no provider is registered for the kind;
    /// providers return [`InstancesPage::default()`] when the kind
    /// exists but has no rows.
    async fn list(
        &self,
        principal: &Principal,
        tenant_id: &str,
        query: InstancesQuery,
    ) -> Result<InstancesPage, InstancesError>;
}

/// Query parameters for the instances endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct InstancesQuery {
    /// Free-text filter — provider decides which fields to match.
    #[serde(default)]
    pub search: Option<String>,
    /// Opaque cursor returned by the previous page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Page size; provider clamps to a sensible max.
    #[serde(default)]
    pub limit: Option<u32>,
    /// Tenant override for super-admin callers (`tenant_id == "*"`).
    #[serde(default)]
    pub tenant: Option<String>,
}

/// One page of instance results.
#[derive(Debug, Clone, Default, Serialize)]
pub struct InstancesPage {
    /// Rows for this page.
    pub items: Vec<ResourceInstance>,
    /// Opaque cursor for the next page; `None` when exhausted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// A single resource instance + its effective ACL summary.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceInstance {
    /// Stable id (e.g. dashboard page id, tool id).
    pub id: String,
    /// Human-readable label rendered by the UI.
    pub label: String,
    /// Owning subject, when the kind models ownership.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<SubjectRef>,
    /// Last-updated marker for sort + display, RFC-3339 string.
    /// `None` when the provider has no notion of update time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Pre-resolved ACL — derived by [`crate::acl::summarise`].
    pub effective_acl: EffectiveAcl,
}

/// Subject in a grant. Wire shape is the same `"team:<slug>"` /
/// `"user:<sub>"` form the rules table stores in `role`, plus a
/// `*` wildcard for tenant-wide grants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SubjectRef {
    /// `"team:<slug>"`.
    Team {
        /// Team slug.
        slug: String,
    },
    /// `"user:<sub>"`.
    User {
        /// Subject identifier (matches `Principal.subject`).
        sub: String,
    },
    /// `"*"` — any authenticated principal in the tenant.
    Wildcard,
}

impl SubjectRef {
    /// Parse a rule's `role` column into a [`SubjectRef`] when it
    /// matches one of the canonical grant subject shapes. Returns
    /// `None` for bare role names (`reader`, `writer`, `admin`,
    /// custom tenant roles) — those aren't grant subjects.
    pub fn parse(role: &str) -> Option<Self> {
        if role == "*" {
            return Some(Self::Wildcard);
        }
        if let Some(slug) = role.strip_prefix("team:") {
            return Some(Self::Team {
                slug: slug.to_string(),
            });
        }
        if let Some(sub) = role.strip_prefix("user:") {
            return Some(Self::User {
                sub: sub.to_string(),
            });
        }
        None
    }
}

/// Permission tier — the three-step ladder Simple mode exposes
/// to operators. Maps to a kind-specific action set via
/// [`crate::acl::actions_for_tier`] / [`crate::acl::tier_for_actions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PermissionTier {
    /// Read-only.
    View,
    /// View + write.
    Edit,
    /// View + write + destructive.
    Manage,
}

/// One bucketed grant in [`EffectiveAcl`].
#[derive(Debug, Clone, Serialize)]
pub struct GrantSummary {
    /// Subject this grant binds.
    pub subject: SubjectRef,
    /// Highest tier held by the subject for this resource.
    pub tier: PermissionTier,
}

/// Coarse share scope used by the page-detail drawer's radios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareScope {
    /// Only the owner has access.
    Private,
    /// Any authenticated member of the tenant has view access.
    Tenant,
    /// Specific teams / users — the `grants` list is authoritative.
    Specific,
}

/// Summary of the rules that apply to a single resource instance.
#[derive(Debug, Clone, Serialize)]
pub struct EffectiveAcl {
    /// Which of the three drawer radios matches today's rules.
    pub share_scope: ShareScope,
    /// Subject → tier list (highest tier per subject).
    pub grants: Vec<GrantSummary>,
    /// `true` when at least one matching rule has a non-`NULL`
    /// `condition` field. These are hand-written legacy rules the
    /// drawer cannot safely round-trip; the UI marks them
    /// read-only.
    pub has_legacy_rules: bool,
}

/// Errors a provider can surface. Kept coarse — handler maps to
/// HTTP status.
#[derive(Debug, thiserror::Error)]
pub enum InstancesError {
    /// Backing store failure.
    #[error("instances provider backend: {0}")]
    Backend(String),
    /// Caller is not allowed to enumerate instances for this kind.
    /// (HTTP 403 — distinct from "kind not registered" → 404.)
    #[error("instances provider forbidden")]
    Forbidden,
}

/// Side-table holding one [`InstancesProvider`] per kind. Kinds
/// that don't opt in simply aren't present; the HTTP handler
/// returns 404 for them.
#[derive(Default)]
pub struct InstancesRegistry {
    providers: RwLock<HashMap<String, Arc<dyn InstancesProvider>>>,
}

impl InstancesRegistry {
    /// New empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider for `kind`. Replaces any prior
    /// registration for the same kind.
    pub fn register(&self, kind: impl Into<String>, provider: Arc<dyn InstancesProvider>) {
        // `RwLock` poisoning would only happen if a previous writer
        // panicked while holding the guard. In that case we'd
        // rather propagate the poisoned state and re-panic at the
        // call site than silently swallow it.
        self.providers
            .write()
            .expect("instances registry poisoned")
            .insert(kind.into(), provider);
    }

    /// Look up a provider for `kind`.
    pub fn get(&self, kind: &str) -> Option<Arc<dyn InstancesProvider>> {
        self.providers
            .read()
            .expect("instances registry poisoned")
            .get(kind)
            .cloned()
    }
}

impl std::fmt::Debug for InstancesRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kinds: Vec<String> = self
            .providers
            .read()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default();
        f.debug_struct("InstancesRegistry")
            .field("kinds", &kinds)
            .finish()
    }
}
