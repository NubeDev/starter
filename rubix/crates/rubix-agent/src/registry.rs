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
use rubix_tools::dashboard::store::InMemoryDashboardStore;
use rubix_tools::dashboard::update::DashboardUpdateTool;
use rubix_tools::dataflow::synth::SynthEmitTool;
use rubix_tools::flow_ops::deploy::FlowDeployTool;
use rubix_tools::flow_ops::duplicate::FlowDuplicateTool;
use rubix_tools::flow_ops::kinds::FlowKindsTool;
use rubix_tools::flow_ops::lint::FlowLintTool;
use rubix_tools::flow_ops::list::FlowListTool;
use rubix_tools::flow_ops::store::{FlowDefStore, InMemoryFlowDefStore};
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
use rubix_tools::team::store::{InMemoryTeamStore, TeamAdminStore};
use rubix_tools::tenant::list::TenantListTool;
use rubix_tools::tenant::store::{InMemoryTenantStore, TenantRow, TenantStore};
use rubix_tools::user::create::UserCreateTool;
use rubix_tools::user::disable::UserDisableTool;
use rubix_tools::user::list::UserListTool;
use rubix_tools::user::store::{InMemoryUserStore, UserAdminStore};
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
use starter_ext_host::ExtensionRegistry;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::NodeBehavior;
use starter_spi::tool::Tool;
use starter_store_postgres::pool::Pool;
use starter_store_warehouse::WarehouseClient;
use tracing::{info, warn};

/// Build the tool registry the agent serves at boot.
pub fn build_tool_registry(
    insights_disk_threshold: u8,
    pg_pool: Option<Pool>,
    warehouse: Option<WarehouseClient>,
    _blob_root: Option<String>,
    extensions: Option<&Arc<ExtensionRegistry>>,
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
        Arc::new(FlowDeployTool::new(flow_store.clone())),
        Arc::new(FlowDuplicateTool::new(flow_store.clone())),
        // ---- user admin -----------------------------------------
        Arc::new(UserListTool::new(user_store.clone())),
        Arc::new(UserCreateTool::new(user_store.clone())),
        Arc::new(UserDisableTool::new(user_store.clone())),
        // ---- tenant + team admin --------------------------------
        Arc::new(TenantListTool::new(tenant_store.clone())),
        Arc::new(TeamCreateTool::new(team_store.clone())),
        Arc::new(TeamAssignTool::new(team_store.clone())),
        // ---- insights admin -------------------------------------
        Arc::new(InsightsRuleListTool::new(insights_store.clone())),
        Arc::new(InsightsRuleCreateTool::new(insights_store.clone())),
        Arc::new(InsightsRuleEnableTool::new(insights_store.clone())),
        Arc::new(InsightsRuleDisableTool::new(insights_store.clone())),
        // ---- dashboard ------------------------------------------
        Arc::new(DashboardGetTool::new(dashboard_store.clone())),
        Arc::new(DashboardListTool::new(dashboard_store.clone())),
        Arc::new(DashboardCreateTool::new(
            dashboard_store.clone(),
            authz_registry.clone(),
        )),
        Arc::new(DashboardUpdateTool::new(dashboard_store.clone())),
        Arc::new(DashboardPatchTool::new(dashboard_store.clone())),
        Arc::new(DashboardDuplicateTool::new(dashboard_store.clone())),
        Arc::new(DashboardDeleteTool::new(dashboard_store.clone())),
        Arc::new(DashboardPageSetTool::new(dashboard_graph.clone())),
    ];

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

fn builtin_kind_behaviors() -> Vec<Arc<dyn NodeBehavior>> {
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
        build_tool_registry(90, None, None, None, None)
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
