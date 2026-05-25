//! Boot-time construction of the authz [`PolicyEngine`] the tools
//! router runs its permission gate against.
//!
//! For v0 the engine is built in-process from a tiny
//! [`StaticRegistry`] declaring the single collection kind the gate
//! enforces (`rubix.tool:invoke`). The store is unused — rules are
//! supplied through the empty [`AuthzConfig`] default, so every
//! authenticated principal is allowed (the
//! `default_policy = true` posture). The 403 path becomes live
//! once policy data lands in `starter_authz_rules`; flipping to a
//! DB-backed [`DbPolicyEngine`] is then a one-line swap. See
//! [docs/design/auth/](../../../docs/design/auth/README.md).

use std::sync::Arc;

use starter_authz::{AuthzConfig, StaticRbacEngine, StaticRegistry};
use starter_spi::authz::{Ownership, PolicyEngine, ResourceRegistry, ResourceSpec};

/// Build the engine the tools-router gate consults. The kind +
/// action strings match
/// [`crate::middleware::authz_gate::TOOL_RESOURCE_KIND`] +
/// [`crate::middleware::authz_gate::TOOL_INVOKE_ACTION`].
pub fn build_engine() -> anyhow::Result<Arc<dyn PolicyEngine>> {
    let registry = Arc::new(StaticRegistry::new());
    registry.register_spec(ResourceSpec::from_static(
        "rubix.tool",
        &["invoke"],
        Ownership::None,
        "Rubix tool",
        "Aggregate resource kind every `rubix.system.*` / `rubix.alert.*` tool dispatch passes through.",
    ));
    // Goal 1, Phase A.1 — SDUI dashboard pages. Authored by
    // operators (and the AI builder), persisted in
    // `dashboards_definitions`, and surfaced as per-page resources
    // through the engine. Per
    // `rubix/docs/scope/dashboards/01-storage.md`: `view` is
    // tenant-scoped, `edit`/`delete` require ownership or
    // `rubix.dashboard:admin`. The `dashboards_seed` boot helper
    // re-registers the same kind on every insert (idempotent via
    // `try_register`); registering it here keeps the early-boot
    // resource list complete before any seed call lands.
    registry.register_spec(ResourceSpec::from_static_tenant_scoped(
        "rubix.dashboard.page",
        &["view", "edit", "delete"],
        Ownership::Subject,
        "Rubix dashboard page",
        "An SDUI page persisted in `dashboards_definitions` and resolved by the page provider.",
    ));
    let cfg = AuthzConfig::default();
    let engine = StaticRbacEngine::from_config(cfg, registry as Arc<dyn ResourceRegistry>)
        .map_err(|e| anyhow::anyhow!("build authz engine: {e}"))?;
    Ok(Arc::new(engine))
}
