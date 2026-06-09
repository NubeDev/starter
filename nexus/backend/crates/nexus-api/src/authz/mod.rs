//! Grant enforcement for the product handlers.
//!
//! Persisted resources are gated by a `starter-authz` grant check on the
//! resource's immutable id, with Postgres RLS as defense-in-depth. The two
//! layers answer different questions: the grant check is "is this principal
//! allowed this action on this specific dashboard?", while RLS is "can this
//! tenant's connection see this row at all?". A row hidden by RLS never reaches
//! a handler, but a row the tenant *can* see still needs a grant before a
//! non-admin may act on it.
//!
//! Resource kinds are tenant-scoped in the registry, so the engine's
//! cross-tenant predicate fires before any rule — the `tenant` on the
//! [`ResourceRef`] must be set for the check to pass.

mod dashboard_instances;

pub use dashboard_instances::DashboardInstancesProvider;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use starter_spi::auth::Principal;
use starter_spi::authz::{Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec};

/// Registry kind for a dashboard. Grants and the engine key on this plus the
/// dashboard's immutable id (never its slug, which is a mutable route alias).
pub const KIND_DASHBOARD: &str = "nexus.dashboard";
/// Registry kind for a datasource.
pub const KIND_DATASOURCE: &str = "nexus.datasource";
/// Registry kind for a saved flow.
pub const KIND_FLOW: &str = "nexus.flow";
/// Registry kind for an alert rule.
pub const KIND_ALERT_RULE: &str = "nexus.alert_rule";

/// Read a resource.
pub const ACTION_VIEW: &str = "view";
/// Modify a resource.
pub const ACTION_EDIT: &str = "edit";
/// Destroy a resource (and, for grants, the highest tier).
pub const ACTION_DELETE: &str = "delete";

const STANDARD_ACTIONS: &[&str] = &[ACTION_VIEW, ACTION_EDIT, ACTION_DELETE];

/// Register the product resource kinds so the engine recognizes them (an
/// unregistered kind is denied as `unknown_resource`) and the grants UI can
/// enumerate the valid (kind, action) targets. All are tenant-scoped, so the
/// engine enforces tenant isolation before evaluating any rule.
pub fn register_nexus_resources(registry: &dyn ResourceRegistry) {
    for kind in [KIND_DASHBOARD, KIND_DATASOURCE, KIND_FLOW, KIND_ALERT_RULE] {
        registry.register(ResourceSpec {
            kind: kind.to_string(),
            actions: STANDARD_ACTIONS.iter().map(|s| s.to_string()).collect(),
            ownership: Ownership::Subject,
            tenant_scoped: true,
            label: kind.to_string(),
            description: format!("Nexus {kind} resource."),
        });
    }
}

/// Authorize `action` on the `(kind, id)` instance for `principal` within
/// `tenant`. Returns `Ok(())` on allow, or a ready `403` response carrying the
/// engine's stable deny reason on deny. The id is the resource's immutable
/// UUID; the tenant binds the engine's cross-tenant predicate.
pub async fn require(
    engine: &dyn PolicyEngine,
    principal: &Principal,
    action: &str,
    kind: &str,
    id: &str,
    tenant: &str,
) -> Result<(), Response> {
    let object = ResourceRef::row(kind, id).with_tenant(tenant);
    let decision = engine.check(principal, action, &object).await;
    if decision.is_allow() {
        return Ok(());
    }
    let reason = match decision {
        starter_spi::authz::Decision::Deny { reason, .. } => reason,
        starter_spi::authz::Decision::Allow { .. } => unreachable!("allow handled above"),
    };
    Err((StatusCode::FORBIDDEN, reason).into_response())
}
