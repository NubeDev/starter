//! Pre-router service construction: config, migrations, pools,
//! extensions, warehouse, tools, flow runtime, MCP surface, scheduler.

use std::sync::Arc;

use anyhow::Result;
use tracing::{info, warn};

use rubix_agent::boot::{self, AgentConfig};
use rubix_agent::{admin, registry, routes};
use starter_changelog_postgres::PgChangeRecorder;
use starter_spi::changelog::ChangeRecorder;
use starter_store_postgres::pool::connect as pg_connect;

/// Everything constructed before the router is assembled.
pub(crate) struct BootedServices {
    pub(crate) cfg: AgentConfig,
    pub(crate) bundle: Arc<starter_i18n::bundle::MessageBundle>,
    pub(crate) mcp_pool: Option<starter_store_postgres::pool::Pool>,
    pub(crate) ext_host_methods: Option<Arc<rubix_agent::extensions::RubixHostMethods>>,
    pub(crate) ext_bundle: Option<boot::ExtensionAdminBundle>,
    pub(crate) warehouse_client: Option<starter_store_warehouse::WarehouseClient>,
    pub(crate) tools: Vec<Arc<dyn starter_spi::tool::Tool>>,
    pub(crate) admin_state: admin::AdminState,
    pub(crate) mcp: boot::mcp::McpSurface,
    pub(crate) flow_runtime: boot::FlowRuntime,
    pub(crate) tools_registrar: routes::RouteRegistrar,
    pub(crate) flow_events_registrar: routes::RouteRegistrar,

    // Leaked task handles — must live for the process lifetime.
    pub(crate) _undo_sweep: Option<tokio::task::JoinHandle<()>>,
    pub(crate) _changelog_sweep: Option<tokio::task::JoinHandle<()>>,
    pub(crate) _scheduler: Option<tokio::task::JoinHandle<()>>,
    pub(crate) _flow_notify: Option<tokio::task::JoinHandle<()>>,
}

pub(crate) async fn boot_services() -> Result<BootedServices> {
    let cfg = AgentConfig::load()?;

    let bundle = Arc::new(rubix_spi::i18n::rubix_bundle()?);
    let catalogue_size: usize = bundle
        .languages()
        .filter_map(|tag| bundle.catalog(tag))
        .map(|cat| cat.messages.len())
        .sum();

    let migrations = boot::apply_migrations(cfg.database_url.as_deref()).await?;
    let _undo_sweep = boot::spawn_undo_sweep(cfg.database_url.as_deref(), cfg.undo.clone()).await?;
    let _changelog_sweep = boot::spawn_changelog_sweep(cfg.database_url.as_deref()).await?;

    let ch_migrations = boot::apply_warehouse_migrations(
        cfg.clickhouse_url.as_deref(),
        cfg.database_url.as_deref(),
        cfg.clickhouse_pg_url.as_deref(),
    )
    .await?;

    let mcp_pool: Option<starter_store_postgres::pool::Pool> = match cfg.database_url.as_deref() {
        Some(dsn) => Some(
            pg_connect(dsn)
                .await
                .map_err(|e| anyhow::anyhow!("connect for mcp pool: {e}"))?,
        ),
        None => None,
    };

    if let Some(pool) = mcp_pool.as_ref() {
        let _t = boot::pool_telemetry::spawn(pool.sqlx().clone(), "rubix-mcp");
    }

    // Extension host methods.
    let ext_host_methods: Option<Arc<rubix_agent::extensions::RubixHostMethods>> = match mcp_pool
        .as_ref()
    {
        Some(pool) => {
            let dashboard_store: Arc<dyn rubix_spi::dashboard::DashboardStore> =
                Arc::new(rubix_store_postgres::PgDashboardStore::new(pool.clone()));
            let engine: Arc<dyn starter_spi::authz::PolicyEngine> = boot::authz::build_engine()?;
            let handler = Arc::new(rubix_agent::extensions::RubixHostMethods::new(
                dashboard_store,
                engine,
            ));
            handler.install_secret_store(rubix_agent::extensions::pick_default_secret_store(env!(
                "CARGO_PKG_NAME"
            )));
            Some(handler)
        }
        None => None,
    };

    // Warehouse — connected *before* the extension admin so the
    // rubix-supplied `WarehouseCleanupProvider` can be registered on the
    // builder with the live warehouse pool. (Per-extension DDL via
    // `create_extension_tables` still runs after the bundle is built,
    // once the registry exists.)
    let warehouse_client = boot::connect_warehouse(
        cfg.warehouse_url.as_deref(),
        mcp_pool.as_ref().map(|p| p.sqlx()),
    )
    .await?;

    let ext_bundle: Option<boot::ExtensionAdminBundle> =
        match (mcp_pool.as_ref(), cfg.extensions.enabled) {
            (Some(pool), true) => {
                // The cleanup providers that need rubix-only knowledge.
                // The warehouse-table reclaimer wires whenever a
                // warehouse is configured; the skill reclaimer wires once
                // rubix grows a live `SkillRegistry` (tracked in
                // `extensions_flow.rs`). The built-in enablement-row +
                // UI/i18n-cache providers auto-register upstream.
                let mut cleanup_providers: Vec<Arc<dyn starter_ext_server::CleanupProvider>> =
                    Vec::new();
                if let Some(wh) = warehouse_client.as_ref() {
                    cleanup_providers.push(Arc::new(
                        rubix_agent::extensions::WarehouseCleanupProvider::new(wh.pool().clone()),
                    ));
                }
                Some(
                    boot::build_extension_admin(
                        &cfg,
                        pool.sqlx(),
                        ext_host_methods.clone(),
                        cleanup_providers,
                    )
                    .await?,
                )
            }
            (Some(_), false) => {
                info!(
                    target: "rubix.boot.extensions",
                    "[extensions].enabled = false — extension host not mounted",
                );
                None
            }
            (None, _) => None,
        };
    let ext_mcp_ctx: Option<boot::mcp::ExtensionMcpContext> =
        ext_bundle.as_ref().map(|b| boot::mcp::ExtensionMcpContext {
            registry: b.registry.clone(),
            process_handles: b.process_handles.clone(),
        });

    let flow_runtime = boot::build_flow_runtime(mcp_pool.clone(), &cfg.flow_runtime).await?;

    // Warehouse (connected above). Spawn pool telemetry and apply the
    // per-extension table DDL now that the sealed registry exists.
    if let Some(wh) = warehouse_client.as_ref() {
        let _t = boot::pool_telemetry::spawn(wh.pool().clone(), "warehouse");
        if let Some(b) = ext_bundle.as_ref() {
            let _ = boot::create_extension_tables(&b.registry, wh).await;
        }
    }

    // Undo substrate.
    let undo_substrate: Option<registry::UndoSubstrate> = match mcp_pool.as_ref() {
        Some(pool) => {
            use starter_changelog::ChangeLog;
            use starter_changelog_postgres::PgChangeLog;
            use starter_undo::cursor_postgres::PgUndoCursor;
            let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool.clone()));
            let log: Arc<dyn ChangeLog> = Arc::new(PgChangeLog::new(pool.clone()));
            let cursor: Arc<dyn starter_undo::UndoCursor> =
                Arc::new(PgUndoCursor::new(pool.clone()));
            Some(registry::UndoSubstrate {
                recorder,
                log,
                cursor,
            })
        }
        None => None,
    };

    let tools = registry::build_tool_registry(
        cfg.insights.disk_warn_threshold,
        mcp_pool.clone(),
        warehouse_client.clone(),
        cfg.blob_root.clone(),
        ext_bundle.as_ref().map(|b| &b.registry),
        undo_substrate,
    );

    // Admin state.
    let mut admin_state = admin::AdminState::empty();
    {
        use std::collections::HashMap;
        let tool_map: HashMap<String, Arc<dyn starter_spi::tool::Tool>> = tools
            .iter()
            .map(|t| (t.definition().name, t.clone()))
            .collect();
        admin_state = admin_state
            .with_tools(Arc::new(tool_map))
            .with_node_behaviors(Arc::new(registry::builtin_kind_behaviors()));
        if let Some(b) = ext_bundle.as_ref() {
            admin_state = admin_state.with_extensions(b.registry.clone());
        }
        if let Some(wh) = warehouse_client.as_ref() {
            let _ = wh;
            let contributions = registry::collect_anomaly_rule_contributions(
                ext_bundle.as_ref().map(|b| &b.registry),
            );
            let rule_registry = rubix_tools::cleaner::adapter::build_registry_with_contributions(
                &tools,
                contributions,
            );
            admin_state = admin_state.with_rules(Arc::new(rule_registry));
        }
    }

    // MCP surface.
    let mcp = boot::mcp::build_mcp_surface(
        mcp_pool.clone(),
        ext_mcp_ctx.as_ref(),
        Some(&flow_runtime),
        Some(tools.clone()),
    )
    .await?;

    // Scheduler.
    let _scheduler = if let Some(pool) = mcp_pool.clone() {
        match boot::spawn_scheduler(pool, mcp.tools.clone(), &cfg.scheduler).await? {
            Some(handle) => {
                info!(
                    target: "rubix.boot.scheduler",
                    seeded = handle.seeded,
                    "durable scheduler running"
                );
                Some(boot::task_watchdog::watch("scheduler", handle.task))
            }
            None => None,
        }
    } else {
        warn!(
            target: "rubix.boot.scheduler",
            "RUBIX_DATABASE_URL unset — scheduled flows will not fire",
        );
        None
    };

    let _flow_notify = boot::spawn_flow_notify(
        cfg.database_url.as_deref(),
        std::sync::Arc::new(|(flow_id, revision, _body)| {
            Box::pin(async move {
                tracing::info!(
                    flow_id = %flow_id,
                    revision = %revision,
                    "flow_notify: reload signal received",
                );
                Ok(())
            })
        }),
    )
    .await?
    .map(|h| boot::task_watchdog::watch("flow_notify", h));

    info!(
        crate_name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        bind = %cfg.bind,
        database_url_set = cfg.database_url.is_some(),
        clickhouse_url_set = cfg.clickhouse_url.is_some(),
        tools = tools.len(),
        mcp_tools = mcp.tools.list().len(),
        skills = rubix_skills::bundled().entries().len(),
        flows = rubix_flows::bundled().entries().len(),
        migrations = migrations.sources_applied,
        migrations_skipped = migrations.skipped,
        ch_migrations_skipped = ch_migrations.skipped,
        i18n_keys = catalogue_size,
        "rubix-agent starting"
    );

    let tools_state = routes::tools::ToolsState::new(tools.clone(), bundle.clone());
    let tools_registrar = routes::tools::registrar(tools_state);

    let flow_events_registrar =
        routes::flow_events::registrar(routes::flow_events::FlowEventsState {
            subscriptions: flow_runtime.subscriptions.clone(),
        });

    Ok(BootedServices {
        cfg,
        bundle,
        mcp_pool,
        ext_host_methods,
        ext_bundle,
        warehouse_client,
        tools,
        admin_state,
        mcp,
        flow_runtime,
        tools_registrar,
        flow_events_registrar,
        _undo_sweep,
        _changelog_sweep,
        _scheduler,
        _flow_notify,
    })
}
