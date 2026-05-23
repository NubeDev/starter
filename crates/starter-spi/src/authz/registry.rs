//! Resource registry: every kind a policy can reference is
//! registered at boot. The trait is the seam; the default impl
//! (a `RwLock<HashMap>`) lives in `starter-authz`.
//!
//! SCOPE.md R4: "Resources are registered, not stringly-
//! discovered." Unknown kinds default to deny.

use serde::{Deserialize, Serialize};

/// Static description of a resource kind. Held by value in the
/// registry; cheap to clone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceSpec {
    /// Stable wire identifier (e.g. `"flows"`, `"users"`).
    pub kind: String,
    /// Closed list of actions defined on this resource. The admin
    /// UI renders exactly these checkboxes — adding a new action
    /// later is a deliberate registry edit, not a typo away.
    pub actions: Vec<String>,
    /// Whether rows of this resource have an owner the engine
    /// should consider for ownership rules.
    pub ownership: Ownership,
    /// Whether rows of this resource carry a `tenant_id` column
    /// (Phase 7a — R11). When `true`, the engine evaluates the
    /// cross-tenant predicate **before** role / condition and
    /// short-circuits to `Deny { reason: "cross_tenant" }` or
    /// `Deny { reason: "no_tenant_binding" }` without consulting
    /// any rule. When `false` (the default — pre-Phase-7
    /// behaviour) the predicate is skipped entirely; this is the
    /// shape for globally-scoped kinds like `users`, `tenants`,
    /// `extensions`.
    #[serde(default)]
    pub tenant_scoped: bool,
    /// Human label for the admin UI; not consumed by the engine.
    pub label: String,
    /// Human description for the admin UI; not consumed by the
    /// engine.
    pub description: String,
}

impl ResourceSpec {
    /// Construct from `&'static str` slices — the common case for
    /// crates registering at boot.
    pub fn from_static(
        kind: &'static str,
        actions: &'static [&'static str],
        ownership: Ownership,
        label: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            actions: actions.iter().map(|s| (*s).to_string()).collect(),
            ownership,
            tenant_scoped: false,
            label: label.to_string(),
            description: description.to_string(),
        }
    }

    /// Tenant-scoped variant — turns the cross-tenant predicate
    /// on for this kind (Phase 7a — R11).
    pub fn from_static_tenant_scoped(
        kind: &'static str,
        actions: &'static [&'static str],
        ownership: Ownership,
        label: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            kind: kind.to_string(),
            actions: actions.iter().map(|s| (*s).to_string()).collect(),
            ownership,
            tenant_scoped: true,
            label: label.to_string(),
            description: description.to_string(),
        }
    }
}

/// Whether the engine should consider an owner subject when
/// evaluating ownership rules.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ownership {
    /// Rows have no owner concept; ownership rules don't apply.
    None,
    /// Rows have a `subject` owner — the engine can match
    /// `principal.subject == object.owner`.
    Subject,
}

/// Registry of resource kinds. Implementations are append-only at
/// boot; double-registration of the same `kind` is a panic (loud
/// failure beats silent shadowing). See SCOPE.md "Extension story".
pub trait ResourceRegistry: Send + Sync + 'static {
    /// Register a resource kind. Panics if `spec.kind` is already
    /// registered.
    fn register(&self, spec: ResourceSpec);

    /// Enumerate every registered spec. Used by the admin UI to
    /// render the permissions grid.
    fn known(&self) -> Vec<ResourceSpec>;

    /// Look up a single spec by kind. Returns `None` if the kind
    /// has not been registered — the engine maps that to
    /// `Decision::Deny { reason: "unknown_resource" }`.
    fn lookup(&self, kind: &str) -> Option<ResourceSpec>;
}
