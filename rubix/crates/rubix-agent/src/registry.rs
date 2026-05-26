//! Boot-time tool registry.
//!
//! Returns the list of `Tool` instances the rubix-agent advertises
//! to MCP / REST callers. One entry per `rubix.<goal>.<verb>` verb
//! the binary has wired. The transport layer reads this once at
//! startup and serves the registry's contents from then on. See
//! [docs/design/tools/](../../docs/design/tools/README.md).
//!
//! ## Backing stores
//!
//! Several verb families (`flow_ops.*`, `user.*`, `tenant.*`,
//! `team.*`) target trait-bound stores whose only impl today is
//! `InMemory*`. The PG-backed swap is documented as a "one-line
//! change in the agent boot wiring" in
//! [`rubix-agent/tests/goal_3_flow_programmer_test.rs`] and in the
//! per-store module docs. Wiring the in-memory variant here is
//! **better than not registering the verbs at all** — the SDK +
//! frontend already call them by id, so leaving them off the
//! registry returns 404 to the SPA and silently breaks every admin
//! surface that consumes them. The in-memory variant returns the
//! correct *shape* (empty list / created row) so the UI renders
//! its "no rows yet" empty state instead of an error toast.
//!
//! Tracked follow-ups (see
//! [`docs/sessions/2026-05-24-tool-registry-gap.md`](../../../docs/sessions/2026-05-24-tool-registry-gap.md)):
//!
//! * Replace `InMemoryUserStore` with a `starter-auth-users`-backed
//!   adapter so `rubix.user.list` reflects the operators created by
//!   `rubix-admin bootstrap-user`.
//! * Replace `InMemoryFlowDefStore` with a PG impl so deploys
//!   survive restart (already seeded from `rubix_flows::bundled()`
//!   below so list/lint/duplicate work read-only today).
//! * Replace `InMemoryTenantStore` / `InMemoryTeamStore` likewise.
//! * Replace `InMemoryChWriter` with a `starter-store-clickhouse`
//!   `ChClient`-backed impl so the seven `rubix.clickhouse.*` verbs
//!   land DDL against the live warehouse.
//! * Replace `InMemoryInsightsStore` with a PG-backed adapter so
//!   insights-rule writes survive restart.

use std::sync::Arc;

use rubix_tools::clickhouse::ch_client_writer::ChClientWriter;
use rubix_tools::clickhouse::mart_create::ClickhouseMartCreateTool;
use rubix_tools::clickhouse::mart_drop::ClickhouseMartDropTool;
use rubix_tools::clickhouse::mart_list::ClickhouseMartListTool;
use rubix_tools::clickhouse::retention_set::ClickhouseRetentionSetTool;
use rubix_tools::clickhouse::rule_list::ClickhouseRuleListTool;
use rubix_tools::clickhouse::rule_write::ClickhouseRuleWriteTool;
use rubix_tools::clickhouse::store::{ChWriter, InMemoryChWriter};
use rubix_tools::clickhouse::tables_list::ClickhouseTablesListTool;
use rubix_spi::dashboard::DashboardStore;
use rubix_tools::dashboard::create::DashboardCreateTool;
use rubix_tools::dashboard::delete::DashboardDeleteTool;
use rubix_tools::dashboard::duplicate::DashboardDuplicateTool;
use rubix_tools::dashboard::get::DashboardGetTool;
use rubix_tools::dashboard::list::DashboardListTool;
use rubix_tools::dashboard::page_set::DashboardPageSetTool;
use rubix_tools::dashboard::store::InMemoryDashboardStore;
use rubix_tools::dashboard::update::DashboardUpdateTool;
use starter_authz::StaticRegistry;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use rubix_tools::flow_ops::deploy::FlowDeployTool;
use rubix_tools::flow_ops::duplicate::FlowDuplicateTool;
use rubix_tools::flow_ops::lint::FlowLintTool;
use rubix_tools::flow_ops::kinds::FlowKindsTool;
use rubix_tools::flow_ops::list::FlowListTool;
use rubix_tools::flow_ops::store::{FlowDefStore, InMemoryFlowDefStore};
use rubix_store_postgres::{PgDashboardStore, PgFlowDefStore};
use rubix_tools::insights::rule_create::InsightsRuleCreateTool;
use rubix_tools::insights::rule_list::InsightsRuleListTool;
use rubix_tools::insights::rule_toggle::{InsightsRuleDisableTool, InsightsRuleEnableTool};
use rubix_tools::insights::store::{InMemoryInsightsStore, InsightsRuleStore};
use rubix_tools::dataflow::synth::SynthEmitTool;
use rubix_tools::system::alert_send::AlertSendTool;
use rubix_tools::system::db::DbTool;
use rubix_tools::system::disk::DiskTool;
use rubix_tools::system::flow_errors::FlowErrorsTool;
use rubix_tools::team::assign::TeamAssignTool;
use rubix_tools::team::create::TeamCreateTool;
use rubix_tools::team::store::{InMemoryTeamStore, TeamAdminStore};
use rubix_tools::tenant::list::TenantListTool;
use rubix_tools::tenant::store::{InMemoryTenantStore, TenantStore};
use rubix_tools::user::create::UserCreateTool;
use rubix_tools::user::disable::UserDisableTool;
use rubix_tools::user::list::UserListTool;
use rubix_tools::user::store::{InMemoryUserStore, UserAdminStore};
use rubix_tools::warehouse::ingest::WarehouseIngestTool;
use rubix_tools::warehouse::clean_minute::WarehouseCleanMinuteTool;
use rubix_tools::warehouse::rollup_15m::WarehouseRollup15mTool;
use rubix_tools::analytics::query::AnalyticsQueryTool;
use starter_flow_spi::node::NodeBehavior;
use starter_spi::tool::Tool;
use starter_store_clickhouse::ChClient;
use starter_store_postgres::pool::Pool;
use tracing::{info, warn};

/// Build the tool registry the agent serves at boot.
///
/// `ch` carries a live ClickHouse client used by tools that write
/// history rows on success (currently just [`DiskTool`]). Passing
/// `None` keeps the verbs runnable without a warehouse — the
/// history write is skipped silently. Order matches the canonical
/// id order so the boot log and any generated OpenAPI / MCP
/// listing are stable across restarts.
pub fn build_tool_registry(
    ch: Option<Arc<ChClient>>,
    insights_disk_threshold: u8,
    pg_pool: Option<Pool>,
) -> Vec<Arc<dyn Tool>> {
    let mut disk = DiskTool::default().with_insights_threshold(insights_disk_threshold);
    let mut warehouse_ingest = WarehouseIngestTool::default();
    let mut warehouse_clean = WarehouseCleanMinuteTool::default();
    let mut warehouse_rollup = WarehouseRollup15mTool::default();
    let analytics_query: Option<Arc<dyn Tool>> = ch.as_ref().map(|client| {
        Arc::new(AnalyticsQueryTool::new(client.clone())) as Arc<dyn Tool>
    });
    if let Some(client) = ch.as_ref() {
        disk = disk.with_history(client.clone());
        warehouse_ingest = warehouse_ingest.with_client(client.clone());
        warehouse_clean = warehouse_clean.with_client(client.clone());
        warehouse_rollup = warehouse_rollup.with_client(client.clone());
    }

    // ---- shared in-memory stores (see module docs) ---------------
    //
    // Single Arc per store so the read + write verbs of a family see
    // the same state within the process. A PG-backed impl can be
    // dropped in here without touching the verb constructors.
    //
    // `flow_store` is the one store that already has a real PG impl
    // (Phase 2 of the live-tick fix). When a pool is wired in we
    // bind it so `flow_ops.deploy` / `.list` / `.duplicate` all
    // target the same `flows_definitions` table the engine-side
    // `flows_seed::seed_and_load` populates on first boot. The
    // in-memory fallback only fires on the laptop / no-DB path —
    // see `rubix/docs/sessions/2026-05-25-tick-counter-r3-and-flow-ops-pg.md`.
    let flow_store: Arc<dyn FlowDefStore> = match pg_pool.as_ref() {
        Some(pool) => Arc::new(PgFlowDefStore::new(pool.clone())),
        None => Arc::new(seed_flow_store()),
    };
    let user_store: Arc<dyn UserAdminStore> = Arc::new(InMemoryUserStore::new());
    let tenant_store: Arc<dyn TenantStore> = Arc::new(InMemoryTenantStore::new());
    let team_store: Arc<dyn TeamAdminStore> = Arc::new(InMemoryTeamStore::new());
    let ch_writer: Arc<dyn ChWriter> = match ch.as_ref() {
        Some(client) => Arc::new(ChClientWriter::new(
            (**client).clone(),
            crate::boot::clickhouse::RUBIX_CH_DATABASE,
        )),
        None => Arc::new(InMemoryChWriter::new()),
    };
    let insights_store: Arc<dyn InsightsRuleStore> = Arc::new(InMemoryInsightsStore::new());
    // Dashboard verbs share the SDUI page provider's store when a PG
    // pool is wired in — that is the single source of truth for
    // `dashboards_definitions`. The laptop / no-DB path falls back
    // to the in-memory impl so `cargo run -p rubix-agent` without
    // Postgres keeps the verbs runnable (writes simply do not
    // survive restart). Without this swap, dashboards authored via
    // the chat / REST tools landed in a process-local in-memory map
    // that the SDUI `/api/v1/ui/resolve` route never reads from
    // (see session note 2026-05-25-dashboard-e2e-three-bugs.md).
    let dashboard_store: Arc<dyn DashboardStore> = match pg_pool.as_ref() {
        Some(pool) => Arc::new(PgDashboardStore::new(pool.clone())),
        None => Arc::new(InMemoryDashboardStore::new()),
    };
    // Shared with `boot::authz`. The dashboard.create tool re-asserts
    // the `rubix.dashboard.page` ResourceSpec on this registry on
    // every successful write; the call is idempotent.
    let authz_registry: Arc<StaticRegistry> = Arc::new(StaticRegistry::new());
    // dashboard.page_set funnels operator slot writes through the R2
    // chokepoint on a `GraphStore`. The registry-local in-memory
    // store stands in for the laptop boot path — the production
    // wiring threads the agent's live `Engine`-backed graph here so
    // every operator write reaches the same chokepoint flows use.
    let dashboard_graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());

    warn!(
        target: "rubix.registry",
        "user/tenant/team/insights verbs are wired against \
         in-memory stores; mutations do not survive restart and do \
         not reflect rows from auth/PG — PG-backed adapters are \
         tracked follow-ups (see registry.rs module docs). The \
         clickhouse.* verbs swap to the live ChClient when one is \
         configured (RUBIX_CH_URL set); without it they fall back \
         to the in-memory writer.",
    );

    let mut tools: Vec<Arc<dyn Tool>> = vec![
        // ---- system / insights ----------------------------------
        Arc::new(disk),
        Arc::new(DbTool),
        Arc::new(FlowErrorsTool::default()),
        Arc::new(AlertSendTool),
        // ---- dataflow (synth) -----------------------------------
        Arc::new(SynthEmitTool::default()),
        // ---- warehouse (ingest + cleaner) -----------------------
        // Append-only L1 writer; targets `rubix.meter_readings_raw`
        // via the boot-applied 0003 migration. See
        // `docs/sessions/data-flow/02-ingest-l1.md`.
        Arc::new(warehouse_ingest),
        // L2 cleaner. One pass per call; the bundled
        // `com.rubix.data-flow.cleaner` flow drives it once per
        // minute. Targets `rubix.meter_readings_1m` via the
        // boot-applied 0004 migration. See
        // `docs/sessions/data-flow/03-clean-to-l2.md`.
        Arc::new(warehouse_clean),
        // L3 rollup. One pass per call; the bundled
        // `com.rubix.data-flow.rollup` flow drives it every 5
        // minutes. Targets `rubix.meter_readings_15m` via the
        // boot-applied 0005 migration. See
        // `docs/sessions/data-flow/05-dashboard-at-scale.md`.
        Arc::new(warehouse_rollup),
        // ---- flow_ops (read + write) ----------------------------
        Arc::new(FlowListTool::new(flow_store.clone())),
        Arc::new(FlowKindsTool::from_behaviors(&builtin_kind_behaviors())),
        Arc::new(FlowLintTool::new()),
        Arc::new(FlowDeployTool::new(flow_store.clone())),
        Arc::new(FlowDuplicateTool::new(flow_store.clone())),
        // ---- user admin (read + write) --------------------------
        Arc::new(UserListTool::new(user_store.clone())),
        Arc::new(UserCreateTool::new(user_store.clone())),
        Arc::new(UserDisableTool::new(user_store.clone())),
        // ---- tenant admin (read-only today) ---------------------
        Arc::new(TenantListTool::new(tenant_store.clone())),
        // ---- team admin (write-only today) ----------------------
        Arc::new(TeamCreateTool::new(team_store.clone())),
        Arc::new(TeamAssignTool::new(team_store.clone())),
        // ---- clickhouse admin (read + write) --------------------
        Arc::new(ClickhouseRuleListTool::new(ch_writer.clone())),
        Arc::new(ClickhouseRuleWriteTool::new(ch_writer.clone())),
        Arc::new(ClickhouseMartListTool::new(ch_writer.clone())),
        Arc::new(ClickhouseMartCreateTool::new(ch_writer.clone())),
        Arc::new(ClickhouseMartDropTool::new(ch_writer.clone())),
        Arc::new(ClickhouseTablesListTool::new(ch_writer.clone())),
        Arc::new(ClickhouseRetentionSetTool::new(ch_writer.clone())),
        // ---- insights admin (read + write) ----------------------
        Arc::new(InsightsRuleListTool::new(insights_store.clone())),
        Arc::new(InsightsRuleCreateTool::new(insights_store.clone())),
        Arc::new(InsightsRuleEnableTool::new(insights_store.clone())),
        Arc::new(InsightsRuleDisableTool::new(insights_store.clone())),
        // ---- dashboard (read + write + runtime slot-set) --------
        // Phase C.5 wires the seven `rubix.dashboard.*` verbs into
        // the same registry the REST tools router and the MCP
        // `tool_registry_snapshot` (see boot/mcp/register.rs) read
        // from. Per R7 the verbs auto-surface to the dashboard-
        // assistant flow's model loop through this snapshot; the
        // flow itself remains the MCP-facing entrypoint.
        Arc::new(DashboardGetTool::new(dashboard_store.clone())),
        Arc::new(DashboardListTool::new(dashboard_store.clone())),
        Arc::new(DashboardCreateTool::new(
            dashboard_store.clone(),
            authz_registry.clone(),
        )),
        Arc::new(DashboardUpdateTool::new(dashboard_store.clone())),
        Arc::new(DashboardDuplicateTool::new(dashboard_store.clone())),
        Arc::new(DashboardDeleteTool::new(dashboard_store.clone())),
        Arc::new(DashboardPageSetTool::new(dashboard_graph.clone())),
        // Phase D.2: the dashboard-assistant YAML is now the real
        // flow rooted at an ai-agent node — the seven verbs above
        // are its `allowed_tools`, surfacing through the
        // `tool_registry_snapshot` boot/mcp/register.rs threads in
        // per R7. The stub `DashboardAssistantStub` it used to
        // dispatch to has been removed.
    ];

    // ---- analytics (read-only) --------------------------------
    // `AnalyticsQueryTool` needs a live `ChClient`; without one we
    // skip registration so the verb does not appear in the
    // catalogue at all. Stage 05 dashboards / report flows rely
    // on this verb to read L3 buckets. See
    // `docs/sessions/data-flow/05-dashboard-at-scale.md` and
    // `docs/design/analytics/`.
    if let Some(t) = analytics_query {
        tools.push(t);
    }
    tools
}

/// Snapshot of the built-in [`NodeBehavior`] instances the kinds
/// verb advertises. Matches the kinds the bundled flows lean on
/// (`starter.flow.counter`, `.log`, `.trigger.explicit`,
/// `.trigger.schedule`) — extending the slice is the one-line change
/// when a new built-in lands. The kinds tool sorts the list before
/// returning, so insertion order here has no observable effect.
fn builtin_kind_behaviors() -> Vec<Arc<dyn NodeBehavior>> {
    vec![
        Arc::new(starter_flow_nodes::counter::Counter::new()),
        Arc::new(starter_flow_nodes::log::Log::new()),
        Arc::new(starter_flow_nodes::trigger_schedule::TriggerSchedule::new()),
    ]
}

/// Seed an [`InMemoryFlowDefStore`] from the bundled flow YAMLs so
/// `rubix.flow_ops.list` returns the canonical six rubix flows on
/// a fresh boot. Each bundled file is inserted as revision 1 at
/// timestamp 0; the deploy verb overwrites these with real
/// revisions once a flow is mutated through the admin surface.
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
        // best-effort: ignore individual seed failures so a single
        // malformed bundled file cannot brick boot.
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
        build_tool_registry(None, 90, None)
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
    fn registry_contains_user_admin_verbs() {
        let names = names();
        for expected in ["rubix.user.list", "rubix.user.create", "rubix.user.disable"] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn registry_contains_tenant_and_team_verbs() {
        let names = names();
        for expected in ["rubix.tenant.list", "rubix.team.create", "rubix.team.assign"] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn registry_contains_every_clickhouse_verb() {
        let names = names();
        for expected in [
            "rubix.clickhouse.rule.list",
            "rubix.clickhouse.rule.write",
            "rubix.clickhouse.mart.list",
            "rubix.clickhouse.mart.create",
            "rubix.clickhouse.mart.drop",
            "rubix.clickhouse.tables.list",
            "rubix.clickhouse.retention.set",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn registry_contains_every_insights_verb() {
        let names = names();
        for expected in [
            "rubix.insights.rule.list",
            "rubix.insights.rule.create",
            "rubix.insights.rule.enable",
            "rubix.insights.rule.disable",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn registry_contains_every_dashboard_verb() {
        let names = names();
        for expected in [
            "rubix.dashboard.get",
            "rubix.dashboard.list",
            "rubix.dashboard.create",
            "rubix.dashboard.update",
            "rubix.dashboard.duplicate",
            "rubix.dashboard.delete",
            "rubix.dashboard.page_set",
        ] {
            assert!(
                names.contains(&expected.to_owned()),
                "registry missing {expected}",
            );
        }
    }

    #[test]
    fn flow_store_is_seeded_from_bundled_flows() {
        // The bundle ships six goal-aligned flow YAMLs (one per
        // goal). The seed should land all six as revision 1 so
        // `rubix.flow_ops.list` is non-empty on first boot.
        let store = seed_flow_store();
        assert!(
            store.len() >= rubix_flows::bundled().files().count(),
            "expected seed_flow_store() to insert one row per bundled flow file",
        );
    }
}
