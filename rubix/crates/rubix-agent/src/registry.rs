//! Boot-time tool registry.
//!
//! Returns the list of `Tool` instances the rubix-agent advertises
//! to MCP / REST callers. One entry per `rubix.<goal>.<verb>` verb
//! the binary has wired.
//!
//! Stage 3 of `rubix/docs/proposal/warehouse-engine-swap.md`
//! removed the ClickHouse-backed `rubix.warehouse.*` verbs, the
//! `rubix.analytics.*` verbs, and the disk tool's ClickHouse
//! history-write side. The seven `rubix.warehouse.*` verbs
//! (rule/mart/tables list, rule.write, mart.create, mart.drop,
//! retention.set) are now wired against TimescaleDB through
//! [`WarehouseClient`] when a warehouse URL is configured.

use std::sync::Arc;

use rubix_spi::dashboard::DashboardStore;
use rubix_store_postgres::{PgDashboardStore, PgFlowDefStore};
use rubix_tools::dashboard::create::DashboardCreateTool;
use rubix_tools::dashboard::delete::DashboardDeleteTool;
use rubix_tools::dashboard::duplicate::DashboardDuplicateTool;
use rubix_tools::dashboard::get::DashboardGetTool;
use rubix_tools::dashboard::list::DashboardListTool;
use rubix_tools::dashboard::page_set::DashboardPageSetTool;
use rubix_tools::dashboard::patch::DashboardPatchTool;
use rubix_tools::dashboard::store::{DashboardReversible, InMemoryDashboardStore};
use rubix_tools::dashboard::update::DashboardUpdateTool;
use rubix_tools::dataflow::synth::SynthEmitTool;
use rubix_tools::flow_ops::deploy::FlowDeployTool;
use rubix_tools::flow_ops::duplicate::FlowDuplicateTool;
use rubix_tools::flow_ops::kinds::FlowKindsTool;
use rubix_tools::flow_ops::lint::FlowLintTool;
use rubix_tools::flow_ops::list::FlowListTool;
use rubix_tools::flow_ops::store::{FlowDefReversible, FlowDefStore, InMemoryFlowDefStore};
use rubix_tools::insights::rule_create::InsightsRuleCreateTool;
use rubix_tools::insights::rule_list::InsightsRuleListTool;
use rubix_tools::insights::rule_toggle::{InsightsRuleDisableTool, InsightsRuleEnableTool};
use rubix_tools::insights::store::{InMemoryInsightsStore, InsightsRuleStore};
use rubix_tools::system::alert_send::AlertSendTool;
use rubix_tools::system::db::DbTool;
use rubix_tools::system::disk::DiskTool;
use rubix_tools::system::flow_errors::FlowErrorsTool;
use rubix_tools::team::assign::TeamAssignTool;
use rubix_tools::team::create::TeamCreateTool;
use rubix_tools::team::delete::TeamDeleteTool;
use rubix_tools::team::store::{InMemoryTeamStore, TeamAdminStore, TeamReversible};
use rubix_tools::team::unassign::TeamUnassignTool;
use rubix_tools::team::update::TeamUpdateTool;
use rubix_tools::tenant::create::TenantCreateTool;
use rubix_tools::tenant::delete::TenantDeleteTool;
use rubix_tools::tenant::list::TenantListTool;
use rubix_tools::tenant::update::TenantUpdateTool;
use rubix_tools::tenant::store::{InMemoryTenantStore, TenantRow, TenantReversible, TenantStore};
use rubix_tools::audit::store::{
    AuditPolicyReversible, AuditPolicyStore, InMemoryAuditPolicyStore,
};
use rubix_tools::audit::policy_list::AuditPolicyListTool;
use rubix_tools::audit::policy_set::AuditPolicySetTool;
use rubix_tools::undo::dispatch::{ActorSource, LocalActor, ReversibleTool, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;
use rubix_tools::undo::redo::UndoRedoTool;
use rubix_tools::user::create::UserCreateTool;
use rubix_tools::user::disable::UserDisableTool;
use rubix_tools::user::enable::UserEnableTool;
use rubix_tools::user::list::UserListTool;
use rubix_tools::user::prefs_set::UserPrefsSetTool;
use rubix_tools::user::role_set::UserRoleSetTool;
use rubix_tools::user::tenant_assign::UserTenantAssignTool;
use rubix_tools::user::store::{InMemoryUserStore, UserAdminStore, UserReversible};
use rubix_tools::cleaner::adapter::{build_registry_with_contributions, ContributedRule};
use rubix_tools::cleaner::tool::CleanerTickTool;
use rubix_tools::warehouse::ingest::WarehouseIngestTool;
use rubix_tools::warehouse::mart_create::WarehouseMartCreateTool;
use rubix_tools::warehouse::mart_drop::WarehouseMartDropTool;
use rubix_tools::warehouse::mart_list::WarehouseMartListTool;
use rubix_tools::warehouse::retention_set::WarehouseRetentionSetTool;
use rubix_tools::warehouse::rule_list::WarehouseRuleListTool;
use rubix_tools::warehouse::rule_write::WarehouseRuleWriteTool;
use rubix_tools::warehouse::tables_list::WarehouseTablesListTool;
use starter_authz::StaticRegistry;
use starter_changelog::ChangeLog;
use starter_ext_host::ExtensionRegistry;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::NodeBehavior;
use starter_spi::changelog::ChangeRecorder;
use starter_spi::tool::Tool;
use starter_store_postgres::pool::Pool;
use starter_store_warehouse::WarehouseClient;
use starter_undo::{ReversibleRegistry, UndoCursor, UndoService};
use tracing::{info, warn};

/// Production undo wiring — the three substrate pieces the
/// reversible tools and the `rubix.undo.{last,redo}` verbs need to
/// reach beyond their own crate. Built once at boot and threaded
/// into [`build_tool_registry`]; passing `None` keeps the registry
/// undo-free (used by the `cargo run -p rubix-agent` no-PG path
/// and by every unit test).
pub struct UndoSubstrate {
    /// Changelog recorder — every reversible mutation goes through
    /// `record_if_reversible(recorder, ...)`. In production this is
    /// `Arc<PgChangeRecorder>`; in tests, `Arc<SqliteChangeRecorder>`.
    pub recorder: Arc<dyn ChangeRecorder>,
    /// Changelog reader — [`UndoService`] walks the per-actor
    /// changes via `log.list(filter_for_actor)` to pick the next
    /// group to replay.
    pub log: Arc<dyn ChangeLog>,
    /// Per-actor redo stack. Production wires `Arc<PgUndoCursor>`
    /// so the stack survives process restarts and crosses agent
    /// instances.
    pub cursor: Arc<dyn UndoCursor>,
}

/// Build the tool registry the agent serves at boot.
pub fn build_tool_registry(
    insights_disk_threshold: u8,
    pg_pool: Option<Pool>,
    warehouse: Option<WarehouseClient>,
    _blob_root: Option<String>,
    extensions: Option<&Arc<ExtensionRegistry>>,
    undo: Option<UndoSubstrate>,
) -> Vec<Arc<dyn Tool>> {
    let disk = DiskTool::new().with_insights_threshold(insights_disk_threshold);

    let flow_store: Arc<dyn FlowDefStore> = match pg_pool.as_ref() {
        Some(pool) => Arc::new(PgFlowDefStore::new(pool.clone())),
        None => Arc::new(seed_flow_store()),
    };
    let user_store: Arc<dyn UserAdminStore> = Arc::new(InMemoryUserStore::new());
    let tenant_store: Arc<dyn TenantStore> =
        Arc::new(InMemoryTenantStore::seeded(vec![TenantRow {
            tenant_id: rubix_spi::dashboard::BUNDLED_TENANT.to_owned(),
            name: "System".to_owned(),
            locale: "en".to_owned(),
        }]));
    let team_store: Arc<dyn TeamAdminStore> = Arc::new(InMemoryTeamStore::new());
    let audit_policy_store: Arc<dyn AuditPolicyStore> = Arc::new(InMemoryAuditPolicyStore::new());
    let insights_store: Arc<dyn InsightsRuleStore> = Arc::new(InMemoryInsightsStore::new());
    let dashboard_store: Arc<dyn DashboardStore> = match pg_pool.as_ref() {
        Some(pool) => Arc::new(PgDashboardStore::new(pool.clone())),
        None => Arc::new(InMemoryDashboardStore::new()),
    };
    let authz_registry: Arc<StaticRegistry> = Arc::new(StaticRegistry::new());
    let dashboard_graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

    warn!(
        target: "rubix.registry",
        "user/tenant/team/insights verbs are wired against in-memory \
         stores; mutations do not survive restart. The rubix.warehouse.* \
         verbs run against TimescaleDB when warehouse_url is configured. \
         The rubix.analytics.* verbs remain removed.",
    );

    // Undo wiring. When `undo` is supplied the per-kind
    // [`Reversible`] impls are mounted on the registry and every
    // reversible tool is wrapped in [`UndoDispatcher`]. When `None`
    // (no-PG dev boot, every unit test) the reversibles still
    // construct against the same stores but skip the dispatcher
    // wrapper, so `rubix.undo.last` is not advertised and writes
    // do not record an undo row.
    let undo_built = undo.as_ref().map(|w| {
        let registry = Arc::new(
            ReversibleRegistry::new()
                .insert(Arc::new(UserReversible::new(user_store.clone())))
                .insert(Arc::new(TeamReversible::new(team_store.clone())))
                .insert(Arc::new(TenantReversible::new(tenant_store.clone())))
                .insert(Arc::new(AuditPolicyReversible::new(audit_policy_store.clone())))
                .insert(Arc::new(DashboardReversible::new(dashboard_store.clone())))
                .insert(Arc::new(FlowDefReversible::new(flow_store.clone()))),
        );
        let service = Arc::new(UndoService::with_cursor(
            w.log.clone(),
            registry.clone(),
            w.cursor.clone(),
        ));
        let actor: Arc<dyn ActorSource> = Arc::new(LocalActor::new());
        (
            registry,
            service,
            actor,
            w.recorder.clone(),
            w.cursor.clone(),
        )
    });

    // Helper closure: wrap a concrete reversible tool in a
    // [`UndoDispatcher`] when undo is wired, otherwise return it as
    // a bare `Arc<dyn Tool>`. Kept inline so the verb list reads
    // top-to-bottom; promoting it to a free function would obscure
    // which tools are reversible.
    //
    // The cursor is threaded through so each successful mutation
    // clears the actor's redo stack (proposal \u00a73.4: "any new
    // mutation by an actor clears that actor's redo stack").
    let wrap_rev = |t: Arc<dyn ReversibleTool>| -> Arc<dyn Tool> {
        match &undo_built {
            Some((registry, _, actor, recorder, cursor)) => Arc::new(UndoDispatcher::with_cursor(
                t,
                registry.clone(),
                recorder.clone(),
                actor.clone(),
                cursor.clone(),
            )),
            None => t,
        }
    };

    let mut tools: Vec<Arc<dyn Tool>> = vec![
        // ---- system / insights ----------------------------------
        Arc::new(disk),
        Arc::new(DbTool),
        Arc::new(FlowErrorsTool::default()),
        Arc::new(AlertSendTool),
        // ---- dataflow (synth) -----------------------------------
        Arc::new(SynthEmitTool::default()),
        // ---- flow_ops (read + write) ----------------------------
        Arc::new(FlowListTool::new(flow_store.clone())),
        Arc::new(FlowKindsTool::from_behaviors(&builtin_kind_behaviors())),
        Arc::new(FlowLintTool::new()),
        wrap_rev(Arc::new(FlowDeployTool::new(flow_store.clone()))),
        wrap_rev(Arc::new(FlowDuplicateTool::new(flow_store.clone()))),
        // ---- user admin -----------------------------------------
        Arc::new(UserListTool::new(user_store.clone())),
        wrap_rev(Arc::new(UserCreateTool::new(user_store.clone()))),
        wrap_rev(Arc::new(UserDisableTool::new(user_store.clone()))),
        wrap_rev(Arc::new(UserEnableTool::new(user_store.clone()))),
        wrap_rev(Arc::new(UserRoleSetTool::new(user_store.clone()))),
        wrap_rev(Arc::new(UserPrefsSetTool::new(user_store.clone()))),
        wrap_rev(Arc::new(UserTenantAssignTool::new(
            user_store.clone(),
            tenant_store.clone(),
        ))),
        // ---- tenant + team admin --------------------------------
        Arc::new(TenantListTool::new(tenant_store.clone())),
        wrap_rev(Arc::new(TenantCreateTool::new(tenant_store.clone()))),
        wrap_rev(Arc::new(TenantUpdateTool::new(tenant_store.clone()))),
        wrap_rev(Arc::new(TenantDeleteTool::new(
            tenant_store.clone(),
            user_store.clone(),
        ))),
        wrap_rev(Arc::new(TeamCreateTool::new(team_store.clone()))),
        wrap_rev(Arc::new(TeamUpdateTool::new(team_store.clone()))),
        wrap_rev(Arc::new(TeamDeleteTool::new(team_store.clone()))),
        wrap_rev(Arc::new(TeamAssignTool::new(team_store.clone()))),
        wrap_rev(Arc::new(TeamUnassignTool::new(team_store.clone()))),
        // ---- audit policy ---------------------------------------
        Arc::new(AuditPolicyListTool::new(audit_policy_store.clone())),
        wrap_rev(Arc::new(AuditPolicySetTool::new(audit_policy_store.clone()))),
        // ---- insights admin -------------------------------------
        Arc::new(InsightsRuleListTool::new(insights_store.clone())),
        Arc::new(InsightsRuleCreateTool::new(insights_store.clone())),
        Arc::new(InsightsRuleEnableTool::new(insights_store.clone())),
        Arc::new(InsightsRuleDisableTool::new(insights_store.clone())),
        // ---- dashboard ------------------------------------------
        Arc::new(DashboardGetTool::new(dashboard_store.clone())),
        Arc::new(DashboardListTool::new(dashboard_store.clone())),
        wrap_rev(Arc::new(DashboardCreateTool::new(
            dashboard_store.clone(),
            authz_registry.clone(),
        ))),
        wrap_rev(Arc::new(DashboardUpdateTool::new(dashboard_store.clone()))),
        wrap_rev(Arc::new(DashboardPatchTool::new(dashboard_store.clone()))),
        wrap_rev(Arc::new(DashboardDuplicateTool::new(dashboard_store.clone()))),
        wrap_rev(Arc::new(DashboardDeleteTool::new(dashboard_store.clone()))),
        Arc::new(DashboardPageSetTool::new(dashboard_graph.clone())),
    ];

    // ---- undo verbs -----------------------------------------
    //
    // Mounted only when undo wiring is supplied. Both verbs share
    // the same [`LocalActor`] so they attribute to the calling
    // principal installed by the tools route's task-local.
    if let Some((_, service, actor, _, _)) = undo_built.as_ref() {
        tools.push(Arc::new(UndoLastTool::new(service.clone(), actor.clone())));
        tools.push(Arc::new(UndoRedoTool::new(service.clone(), actor.clone())));
    }

    if let Some(wh) = warehouse {
        tools.push(Arc::new(WarehouseIngestTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseRuleListTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseMartListTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseTablesListTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseRuleWriteTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseMartCreateTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseMartDropTool::new(wh.clone())));
        tools.push(Arc::new(WarehouseRetentionSetTool::new(wh.clone())));
        // Cleaner tick — driven by the bundled `com.rubix.cleaner`
        // flow on a 60s schedule. Registry is the three builtins
        // (NaN → Spike → Stuck) plus one `ToolAnomalyRule` per
        // `contributes.anomaly_rules[]` entry across every Validated
        // extension. Contributed rules whose `tool_id` does not
        // resolve in the just-built `tools` list are dropped with a
        // warn log inside the builder.
        let contributions = collect_anomaly_rule_contributions(extensions);
        let rule_registry = build_registry_with_contributions(&tools, contributions);
        info!(
            target: "rubix.registry",
            rule_count = rule_registry.len(),
            rules = ?rule_registry.ids().collect::<Vec<_>>(),
            "cleaner rule registry built",
        );
        tools.push(Arc::new(CleanerTickTool::new(wh, rule_registry)));
    }

    tools
}

/// Project every Validated extension's
/// `contributes.anomaly_rules[]` entry into the
/// rubix-tools-internal [`ContributedRule`] shape. Declaration
/// order is preserved across extensions in the order they appear
/// in the sealed registry; the builder will re-sort by
/// `(priority, declaration index)`.
///
/// Exposed for the Phase B gate integration test
/// (`tests/extension_anomaly_rule_gate_test.rs`) so the test can
/// drive the same projection the boot path uses without
/// re-implementing it.
pub fn collect_anomaly_rule_contributions(
    extensions: Option<&Arc<ExtensionRegistry>>,
) -> Vec<ContributedRule> {
    let Some(registry) = extensions else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for record in registry.iter_validated() {
        let Some(manifest) = record.manifest.as_ref() else {
            continue;
        };
        for entry in &manifest.contributes.anomaly_rules {
            out.push(ContributedRule {
                id: entry.id.clone(),
                tool_id: entry.tool_id.clone(),
                priority: entry.priority,
            });
        }
    }
    out
}

/// Built-in flow node behaviours the engine ships. Exposed so
/// the admin introspection projection can walk the same list the
/// boot path uses to seed the live
/// [`NodeKindRegistry`](starter_flow::registry::NodeKindRegistry).
pub fn builtin_kind_behaviors() -> Vec<Arc<dyn NodeBehavior>> {
    vec![
        Arc::new(starter_flow_nodes::counter::Counter::new()),
        Arc::new(starter_flow_nodes::log::Log::new()),
        Arc::new(starter_flow_nodes::trigger_schedule::TriggerSchedule::new()),
    ]
}

fn seed_flow_store() -> InMemoryFlowDefStore {
    let store = InMemoryFlowDefStore::new();
    let mut seeded = 0usize;
    for entry in rubix_flows::bundled().files() {
        let Some(body) = entry.contents_utf8() else {
            continue;
        };
        let Some(flow_id) = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| format!("com.rubix.{s}"))
        else {
            continue;
        };
        match futures::executor::block_on(store.insert_revision(&flow_id, body, 0)) {
            Ok(_) => seeded += 1,
            Err(err) => warn!(
                target: "rubix.registry",
                %flow_id, error = %err,
                "flow seed failed",
            ),
        }
    }
    info!(
        target: "rubix.registry",
        seeded,
        "seeded in-memory flow store from rubix_flows::bundled()",
    );
    store
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        build_tool_registry(90, None, None, None, None, None)
            .iter()
            .map(|t| t.definition().name)
            .collect()
    }

    #[test]
    fn registry_contains_disk_tool() {
        assert!(names().contains(&"rubix.system.disk".to_owned()));
    }

    #[test]
    fn registry_contains_every_wired_system_tool() {
        let names = names();
        for expected in [
            "rubix.system.disk",
            "rubix.system.db",
            "rubix.system.flow_errors",
            "rubix.alert.send",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn registry_contains_dataflow_synth_emit() {
        assert!(names().contains(&"rubix.dataflow.synth.emit".to_owned()));
    }

    #[test]
    fn registry_contains_flow_ops_quartet() {
        let names = names();
        for expected in [
            "rubix.flow_ops.list",
            "rubix.flow_ops.kinds",
            "rubix.flow_ops.lint",
            "rubix.flow_ops.deploy",
            "rubix.flow_ops.duplicate",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn flow_store_is_seeded_from_bundled_flows() {
        let store = seed_flow_store();
        assert!(
            store.len() >= rubix_flows::bundled().files().count(),
            "expected seed_flow_store() to insert one row per bundled flow file",
        );
    }
}
