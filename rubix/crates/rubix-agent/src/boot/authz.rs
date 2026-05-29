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

use starter_authz::audit::DecisionSink;
use starter_authz::store::PostgresPolicyStore;
use starter_authz::{DbPolicyEngine, StaticRegistry};
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
