//! Rubix backend binary.
//!
//! Wiring composition site. Boots `starter-observability`, loads the
//! layered [`boot::AgentConfig`], applies the rubix-owned Postgres +
//! ClickHouse migrations, builds the tool registry (threading a live
//! [`starter_store_warehouse::ChClient`] through the disk tool), and
//! composes one [`axum::Router`] from four sub-routers:
//!
//!   - `GET  /healthz`
//!   - `POST /api/v1/auth/{login,logout,me}` (via
//!     [`starter_auth_users::routes::auth_router`])
//!   - `POST /api/v1/tools/{tool_id}` (gated by
//!     [`middleware::gate_tools`] + audited by
//!     [`middleware::changelog_layer`])
//!   - `POST /api/v1/mcp` (the starter-mcp surface)
//!
//! This file *wires only*. Any logic that isn't pure glue belongs
//! in `rubix-tools` or upstream in starter. See
//! [docs/design/agent/](../../docs/design/agent/README.md).

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tracing::{info, warn};

use rubix_agent::boot::{self, AgentConfig};
use rubix_agent::{health, middleware, openapi as rubix_openapi_mod, registry, routes};
use starter_changelog_postgres::PgChangeRecorder;
use starter_spi::changelog::ChangeRecorder;
use starter_store_postgres::pool::connect as pg_connect;
#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

    // Install a panic hook that logs every panic via `tracing`
    // before the default hook runs. Without this, a panic inside
    // a `tokio::spawn`'d task is silently caught by the runtime
    // and printed to stderr in a format the operator can easily
    // miss (and that does not interleave with the structured
    // tracing log). Wrapping `take_hook` chains us in front of
    // the default so the standard backtrace still fires.
    //
    // This was wired in after the freeze investigation surfaced
    // a `tracing-subscriber` "tried to clone a span that already
    // closed" assertion that was sitting silently in
    // `/tmp/rubix-agent.log` for hours. With the hook installed,
    // any future panic gets a `target="rubix.panic"` line that
    // stands out alongside normal events.
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "<unknown>".into());
            let payload = info
                .payload()
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_owned())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic payload>".into());
            tracing::error!(
                target: "rubix.panic",
                location = %loc,
                thread = %std::thread::current().name().unwrap_or("<unnamed>"),
                payload = %payload,
                "panic caught",
            );
            default_hook(info);
        }));
    }

    // Runtime liveness canary. Bumps an atomic every 1s; the
    // `/livez` route reads it. If the atomic stops advancing the
    // tokio runtime itself is wedged (worker threads parked on
    // futex, no tasks making progress) and `/livez` returns 503
    // with the staleness. Distinct from `/healthz` (TCP listener
    // alive) and `/readyz` (DB pool alive). See
    // `boot::runtime_canary`.
    let (runtime_canary, runtime_canary_task) = boot::runtime_canary::spawn();
    let _runtime_canary_task = boot::task_watchdog::watch("runtime_canary", runtime_canary_task);

    // On-demand tokio runtime metrics dump. `kill -USR1 <pid>`
    // emits one `target=rubix.runtime_metrics` line with
    // num_workers / num_alive_tasks. Pairs with the canary above:
    // canary says *whether* the runtime is wedged, this says
    // *how*. See `boot::runtime_metrics`.
    let _runtime_metrics_task = match boot::runtime_metrics::spawn() {
        Ok(h) => Some(boot::task_watchdog::watch("runtime_metrics", h)),
        Err(e) => {
            warn!(
                target: "rubix.boot.runtime_metrics",
                error = %e,
                "failed to install SIGUSR1 handler — metrics dump disabled",
            );
            None
        }
    };

    let cfg = AgentConfig::load()?;

    let bundle = Arc::new(rubix_spi::i18n::rubix_bundle()?);
    let catalogue_size: usize = bundle
        .languages()
        .filter_map(|tag| bundle.catalog(tag))
        .map(|cat| cat.messages.len())
        .sum();

    let migrations = boot::apply_migrations(cfg.database_url.as_deref()).await?;
    // The sweep handle is intentionally leaked into the process
    // lifetime — `health::serve` blocks until shutdown, at which
    // point the runtime dropping aborts every task. See
    // [`boot::undo_sweep`] for the cadence + bound contract.
    let _undo_sweep = boot::spawn_undo_sweep(cfg.database_url.as_deref(), cfg.undo.clone()).await?;
    // Per-kind retention sweep for `starter_changes` — the audit
    // table's counterpart to `_undo_sweep` above. Kinds with no
    // policy row, or with `max_age_days = NULL`, are skipped
    // (today's implicit-unbounded baseline). The `changelog_policy`
    // seed migration above pins `user`/`team` to NULL = keep
    // forever. See `rubix/docs/proposal/audit-log.md`.
    let _changelog_sweep = boot::spawn_changelog_sweep(cfg.database_url.as_deref()).await?;
    // Stage 3 of warehouse-engine-swap: the ClickHouse engine is
    // gone. The warehouse capability crate will be rebuilt on
    // TimescaleDB in a follow-up stage; for now the boot wiring
    // here is a no-op.
    let ch_migrations = boot::apply_warehouse_migrations(
        cfg.clickhouse_url.as_deref(),
        cfg.database_url.as_deref(),
        cfg.clickhouse_pg_url.as_deref(),
    )
    .await?;
    // Reuse a single PG pool for the MCP surface (flows_definitions
    // seed + load) so we don't open a second connection pool just
    // to read flow YAMLs. `None` is the laptop path — MCP falls
    // back to the embedded bundle.
    let mcp_pool: Option<starter_store_postgres::pool::Pool> = match cfg.database_url.as_deref() {
        Some(dsn) => Some(
            pg_connect(dsn)
                .await
                .map_err(|e| anyhow::anyhow!("connect for mcp pool: {e}"))?,
        ),
        None => None,
    };

    // Observability: stream pool stats every 30s so a future
    // "agent stopped responding" investigation can correlate the
    // freeze with pool saturation without an out-of-band
    // `pg_stat_activity` capture. The handle is intentionally
    // leaked into the process lifetime — runtime shutdown drops
    // the task. See `boot::pool_telemetry`.
    if let Some(pool) = mcp_pool.as_ref() {
        let _t = boot::pool_telemetry::spawn(pool.sqlx().clone(), "rubix-mcp");
    }

    // SCOPE OQ-4: build the extension admin BEFORE the MCP surface so
    // the surface can emit one MCP tool per
    // `contributes.tools[]` entry alongside the bundled `FlowAsTool`
    // entries. The ordering is load-bearing: starter-mcp's
    // `ToolRegistry` is wrapped in `Arc` once `build_mcp_surface`
    // returns, so any extension tool not registered here is silently
    // missing from `tools/list`. The PG pool acquired here is reused
    // below for the auth + changelog sandwich — keeping the
    // connection-pool count at one.
    // Build the rubix-side `HostMethodHandler` BEFORE the
    // extension admin so autostarted process-flavour extensions
    // inherit it from their first spawn. `RubixHostMethods`
    // captures `Arc<PgDashboardStore>` over the same `mcp_pool`
    // the SDUI page provider writes through, and the
    // `StaticRbacEngine` built here (the same one `gate_tools`
    // consults later — cloned cheaply because it's Arc-backed).
    // Skipped when no PG pool is configured (laptop dev path) —
    // process-flavour host calls then fall back to the
    // supervisor's not-implemented default.
    let ext_host_methods: Option<Arc<rubix_agent::extensions::RubixHostMethods>> =
        match mcp_pool.as_ref() {
            Some(pool) => {
                let dashboard_store: Arc<dyn rubix_spi::dashboard::DashboardStore> =
                    Arc::new(rubix_store_postgres::PgDashboardStore::new(pool.clone()));
                let engine: Arc<dyn starter_spi::authz::PolicyEngine> =
                    boot::authz::build_engine()?;
                // Build the bare handler now — warehouse + event-bus
                // deps are bolted on below once the warehouse client
                // and a shared `RubixEventBus` are in hand. Likewise
                // the sealed `ExtensionRegistry` is installed inside
                // `build_extension_admin` (it doesn't exist yet).
                //
                // Secret store: pick the keyring backing when the
                // platform service answers a probe (developer
                // workstations); fall back to env-var lookup
                // otherwise (CI, headless containers, …). Operators
                // who need a different backing (Vault, age-encrypted
                // file) can swap in their own `SecretStore` impl
                // and call `install_secret_store(...)` themselves.
                let handler = Arc::new(rubix_agent::extensions::RubixHostMethods::new(
                    dashboard_store,
                    engine,
                ));
                handler.install_secret_store(
                    rubix_agent::extensions::pick_default_secret_store(env!("CARGO_PKG_NAME")),
                );
                Some(handler)
            }
            None => None,
        };
    let ext_bundle: Option<boot::ExtensionAdminBundle> =
        match (mcp_pool.as_ref(), cfg.extensions.enabled) {
            (Some(pool), true) => Some(
                boot::build_extension_admin(&cfg, pool.sqlx(), ext_host_methods.clone()).await?,
            ),
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

    // Build the always-on flow runtime BEFORE the MCP surface so
    // the `FlowAsTool` engine can share its durable
    // `NodeStateStore` and its `FlowEventSink` (the per-flow SSE
    // broadcast registry). Without this hand-off, scheduled
    // / MCP-triggered runs use a noop state store (counter state
    // never persists) and their events never reach SSE.
    //
    // Reuses the `mcp_pool` opened above so we keep the
    // connection-pool count at one (the node_state seam used to
    // open a second pool from the DSN).
    let flow_runtime = boot::build_flow_runtime(mcp_pool.clone(), &cfg.flow_runtime).await?;

    // Build the REST tool registry BEFORE the MCP surface so the
    // exact same `Vec<Arc<dyn Tool>>` is threaded into both — the
    // `ai-agent` node dispatches against the SAME store instances
    // the REST `/api/v1/tools/*` router serves. Without the shared
    // list, every `InMemory*Store`-backed tool family got a second
    // store the REST writes never reached.
    // Clones taken here mirror the previous boot order: `ch_client`
    // is moved into the tool list; the SDUI + explorer routers need
    // their own handles.
    // Wire the Timescale warehouse plane (samples hypertable +
    // analytics templates). Skipped silently when `warehouse_url`
    // is unset — the agent still boots, dashboards just render
    // empty values.
    let warehouse_client = boot::connect_warehouse(
        cfg.warehouse_url.as_deref(),
        mcp_pool.as_ref().map(|p| p.sqlx()),
    )
    .await?;
    if let Some(wh) = warehouse_client.as_ref() {
        let _t = boot::pool_telemetry::spawn(wh.pool().clone(), "warehouse");
        // Apply extension-declared `contributes.warehouse_tables[]`
        // DDL once per boot. Idempotent (CREATE TABLE IF NOT EXISTS).
        // Skipped silently when the extension bundle hasn't been
        // built yet (e.g. extensions disabled).
        if let Some(b) = ext_bundle.as_ref() {
            let _ = boot::create_extension_tables(&b.registry, wh).await;
        }
    }

    // Build the undo substrate the reversible tools will be wrapped
    // around. Only landed when a PG pool is wired — the substrate
    // needs both the changelog (recorder + read-side query) and a
    // durable cursor, neither of which has a useful in-memory
    // approximation at boot scale.
    //
    // The substrate is constructed *before* `build_tool_registry`
    // because the registry inlines the `UndoDispatcher` wrap on the
    // reversible tools and appends `rubix.undo.{last,redo}`. See
    // `registry::UndoSubstrate` for the three pieces.
    let undo_substrate: Option<registry::UndoSubstrate> = match mcp_pool.as_ref() {
        Some(pool) => {
            use starter_changelog::ChangeLog;
            use starter_changelog_postgres::PgChangeLog;
            use starter_undo::cursor_postgres::PgUndoCursor;
            // Migration for `starter_undo_cursors` is applied in
            // `boot::migrations::apply_migrations` alongside
            // every other rubix-owned source so the schema is
            // provisioned atomically with the rest of the boot.
            let recorder: Arc<dyn ChangeRecorder> =
                Arc::new(PgChangeRecorder::new(pool.clone()));
            let log: Arc<dyn ChangeLog> = Arc::new(PgChangeLog::new(pool.clone()));
            let cursor: Arc<dyn starter_undo::UndoCursor> =
                Arc::new(PgUndoCursor::new(pool.clone()));
            Some(registry::UndoSubstrate { recorder, log, cursor })
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

    // Admin introspection state — populated as each registry
    // becomes available below. The router itself is mounted at
    // the end of the auth-gated block so the
    // [`with_role(Role::Admin)`] gate wraps every read.
    // See docs/design/admin/README.md.
    let mut admin_state = rubix_agent::admin::AdminState::empty();
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
            // The same rule-set the cleaner runs against — built
            // here a second time as a read-only projection for
            // the admin surface. Cheap (per-rule construction is
            // trivial); avoids threading the constructed registry
            // out of `build_tool_registry`.
            let _ = wh;
            let contributions = registry::collect_anomaly_rule_contributions(
                ext_bundle.as_ref().map(|b| &b.registry),
            );
            let rule_registry =
                rubix_tools::cleaner::adapter::build_registry_with_contributions(
                    &tools, contributions,
                );
            admin_state = admin_state.with_rules(Arc::new(rule_registry));
        }
    }

    let mcp = boot::mcp::build_mcp_surface(
        mcp_pool.clone(),
        ext_mcp_ctx.as_ref(),
        Some(&flow_runtime),
        Some(tools.clone()),
    )
    .await?;

    // Phase D.2 — durable cron scheduler. Wires only when a PG
    // pool is present (the scheduler's claim/dispatch loop is
    // table-driven; no table means nothing to claim) and when
    // `[scheduler].enabled` is true (default). The handle is
    // intentionally leaked into the process lifetime — the
    // tokio runtime aborts the tick task on shutdown via the
    // same pattern as `_undo_sweep` above.
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
            // Phase D.1 lands the listener + the seed/load contract.
            // The actual `FlowRegistry::register` reload wires in
            // alongside the goal-3 flow-programmer verbs in a
            // subsequent stage — for now we log so the channel
            // wiring is observable end-to-end.
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
    // Clone the optional `ChClient` for the SDUI query engine before
    // the registry consumes the original. Phase B.2: the SDUI
    // `QueryEngine` honours `ch:<table>` prefixes against the same
    // warehouse the disk tool persists history into.
    // (Tool registry + the `ch_client_for_*` clones were moved
    // above so MCP and REST share one `Vec<Arc<dyn Tool>>`.)

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

    // Phase C.2 — flow_runtime was built earlier so its
    // `NodeStateStore` + `FlowEventSink` could be attached to the
    // MCP-side engine. Reuse the existing handle here to wire the
    // SSE route.
    let flow_events_registrar =
        routes::flow_events::registrar(routes::flow_events::FlowEventsState {
            subscriptions: flow_runtime.subscriptions.clone(),
        });
    // Surface the runtime via the same `_` leak pattern as the other
    // always-on boot pieces.
    let _flow_runtime = flow_runtime;

    // ----------------------------------------------------------------
    // Compose. The auth + authz + changelog sandwich layers in only
    // when a database is configured; without a DSN the binary still
    // serves /healthz + /api/v1/mcp + an ungated /api/v1/tools/*
    // surface so a developer can drive the agent on a laptop. The
    // production smoke path always sets RUBIX_DATABASE_URL.
    //
    // `app` is a [`routes::RouteRegistrar`] so every
    // rubix-agent-owned mount records its (method, path,
    // description) tuple alongside the live router. Upstream
    // routers (`starter-auth-users`, `starter-ext-server`,
    // `starter-warehouse-explorer`, `starter-sdui-routes`, MCP)
    // are merged via `merge_external` — they keep their own
    // OpenAPI projection; this catalog reflects only what
    // rubix-agent owns. The catalog is projected into
    // `GET /api/v1/admin/openapi.json` at the end of the wire-up.
    // ----------------------------------------------------------------
    // The MCP router builds its own `/mcp` route; nest under
    // `/api/v1` so the production surface lands at
    // `POST /api/v1/mcp` per docs/design/mcp-ux/README.md.
    let mcp_routes = Router::new().nest("/api/v1", mcp.router);
    let openapi_doc_registrar =
        routes::openapi_doc::openapi_registrar(rubix_openapi_mod::rubix_openapi());
    let mut app: routes::RouteRegistrar = routes::RouteRegistrar::new()
        .merge(health::healthz_registrar())
        .merge(health::livez_registrar(runtime_canary.clone()))
        .merge_external(mcp_routes)
        .merge(openapi_doc_registrar)
        // SSE flow-events route. CSRF-exempt (mirrors the
        // extensions-events route): `EventSource` cannot send a CSRF
        // token and `text/event-stream` GETs carry no body. AuthN
        // still gates the route under the standard `with_principal`
        // sandwich when a DSN is set; without a DSN the laptop dev
        // path leaves it open alongside the tools router.
        .merge(flow_events_registrar);

    // Warehouse explorer + status routers were removed in stage 3
    // of warehouse-engine-swap; the ClickHouse-backed REST surface
    // is gone. A TimescaleDB equivalent will land in a follow-up
    // stage once the warehouse capability crate is rebuilt.

    if let Some(dsn) = cfg.database_url.as_deref() {
        let pool = pg_connect(dsn)
            .await
            .map_err(|e| anyhow::anyhow!("connect to RUBIX_DATABASE_URL: {e}"))?;
        // Telemetry for the auth/changelog/explorer pool. Distinct
        // label from `rubix-mcp` so a starved auth pool (the one
        // that gates every `with_principal`-wrapped route) is
        // visible separately in the log.
        let _t_auth = boot::pool_telemetry::spawn(pool.sqlx().clone(), "rubix-auth");
        // Readiness probe — uses this pool because every
        // auth-gated request hits it. If this pool is saturated,
        // /readyz returns 503 within 1 s instead of timing out
        // alongside every browser request.
        app = app.merge(health::readyz_registrar(pool.sqlx().clone()));
        let auth = boot::build_auth(pool.clone());
        let auth_routes = routes::auth::auth_router(auth.state);
        let engine = boot::authz::build_engine()?;

        // `/api/warehouse/status` removed alongside the ClickHouse
        // engine deletion (stage 3 of warehouse-engine-swap).

        // Warehouse explorer — phase 4 of warehouse-engine-swap.
        // Mounts the read-only sql-studio-style surface at
        // `/api/warehouse/explorer/*` against the **warehouse** pool
        // (the OLTP pool would expose `_sqlx_migrations_*`,
        // `sessions`, `flow_*`, … and hide extension warehouse
        // tables). Gated by `with_principal` + `with_role(Role::Admin)`
        // to match the old explorer's posture. The URL `/ch` suffix
        // was dropped now that the backend is Postgres/TimescaleDB.
        if let Some(wh_client) = warehouse_client.clone() {
            let explorer_router =
                starter_warehouse_explorer::router_with_auth(wh_client, auth.authenticator.clone());
            app = app.merge_external(explorer_router);
        }

        // Phase C.2 — mount the extension-host admin router. The
        // lifecycle endpoints (`/extensions`, `/extensions/{id}`,
        // `/extensions/{id}/{enable,disable,events}`) are sandwiched by
        // upstream `with_principal` + `with_role(Role::Admin)` inside
        // `starter_ext_server::router_with_auth`, which is the
        // admin-only authz gate the rubix surface reuses for operator
        // routes. The UI bundle + i18n catalogue routes remain
        // unauthed (see starter_ext_server::router module docs). The
        // host is skipped entirely when `[extensions].enabled = false`
        // so integration tests can opt out without a stub PG table.
        // The extension admin was built earlier (above the MCP
        // surface) so its registry + supervisor handles could be
        // threaded into `tools/list`. Here we just consume the
        // pre-built `ExtensionAdmin` into the auth-gated router.
        if let Some(bundle) = ext_bundle {
            // Admin lifecycle routes (list/detail/enable/disable/events).
            let ext_router: Router =
                starter_ext_server::router_with_auth(bundle.admin, auth.authenticator.clone());
            app = app.merge_external(Router::new().nest("/api/v1", ext_router));

            // Dispatcher-backed per-extension REST adapter (row 5 boot
            // wiring). Builtin-flavour extensions get their contributed
            // tools/REST verbs mounted here; the `RubixCapabilityFactory`
            // is the host-side seam that hands real `WarehouseRead` +
            // `EventBus` backends to each per-call `Ctx`. Today rubix
            // ships no builtin extensions, so `rest_router` produces an
            // empty `Router` — the wiring still lands the seam so the
            // first builtin extension to land later is dispatched
            // through real backends instead of fail-closed stubs.
            if let Some(wh_client) = warehouse_client.clone() {
                use rubix_agent::extensions::{
                    with_caller_identity, CompositeRestDispatcher, RubixCapabilityFactory,
                    RubixEventBus, EXTENSION_REST_REQUEST_TIMEOUT,
                };
                use starter_ext_host::TemplateRegistry;
                use starter_ext_sdk::builtin::BuiltinTable;
                use starter_ext_server::{
                    rest_router, BuiltinRestDispatcher, CapabilityFactory, DispatcherCache,
                    KindCacheRegistry, ProcessRestDispatcher, RestDispatcher, RestRouterOptions,
                };
                use starter_server::auth::with_principal;

                let table = Arc::new(BuiltinTable::new());
                let mut tmpl = TemplateRegistry::builtin();
                // Fold every validated extension's
                // `contributes.warehouse_templates[]` into the host
                // registry so the extension-substrate
                // `ctx.warehouse_read().query(...)` path (and the
                // process-flavour `warehouse.query` proxy tool) can
                // resolve names like `com.rubix.example.customers_by_country`.
                for record in bundle.registry.iter_validated() {
                    match tmpl.extend_from_record(record) {
                        Ok(n) if n > 0 => {
                            info!(
                                target: "rubix.boot.extensions",
                                extension = %record.id_hint,
                                templates = n,
                                "registered contributed warehouse templates",
                            );
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!(
                                target: "rubix.boot.extensions",
                                extension = %record.id_hint,
                                error = %e,
                                "failed to register contributed warehouse templates",
                            );
                        }
                    }
                }
                let template_registry = Arc::new(tmpl);
                let event_bus = Arc::new(RubixEventBus::new());
                // Wire the Row-5 dashboard + authz backends from the
                // already-built rubix primitives:
                // - `PgDashboardStore` over the boot PG pool (same
                //   pool the SDUI page provider and the
                //   `rubix.dashboard.*` tools dispatch against, so
                //   extension writes are visible to the existing UI).
                // - The `StaticRbacEngine` constructed in `boot::authz`
                //   (same engine the `gate_tools` middleware consults).
                //   Cloned cheaply — both are `Arc`-backed.
                let dashboard_store: Arc<dyn rubix_spi::dashboard::DashboardStore> =
                    Arc::new(rubix_store_postgres::PgDashboardStore::new(pool.clone()));
                // Share the same warehouse client + template registry +
                // event bus with `RubixHostMethods` so the process-flavour
                // host methods (`warehouse.query` / `event_bus.publish`)
                // dispatch through the same primitives the builtin-flavour
                // factory uses. Single source of truth for both flavours.
                if let Some(host_methods) = ext_host_methods.as_ref() {
                    host_methods
                        .install_warehouse(wh_client.clone(), template_registry.clone());
                    host_methods.install_event_bus(event_bus.clone());
                }
                // Hand the template registry to the admin
                // introspection state so `GET /api/v1/admin/registry/templates`
                // sees the same set the warehouse-read path
                // resolves against.
                admin_state = admin_state.with_templates(template_registry.clone());
                let factory: Arc<dyn CapabilityFactory> = Arc::new(
                    RubixCapabilityFactory::new(wh_client, template_registry, event_bus)
                        .with_extension_registry(bundle.registry.clone())
                        .with_dashboard_store(dashboard_store)
                        .with_authz_engine(engine.clone()),
                );
                // v0 opt-in cache: walk every validated extension's
                // bundle and pick up `kinds/*.cache.yaml` sidecars
                // into a single KindCacheRegistry shared by the
                // builtin and process dispatchers. See
                // rubix/docs/sessions/cache-v0-progress.md and
                // rubix/docs/proposal/fe-cache-opt-in.md "Minimum
                // viable v0". Per-tenant cap left at the default
                // (10k entries / tenant) until production data tells
                // us otherwise.
                let cache_layer = starter_cache::CacheLayer::new(
                    starter_cache::LayerConfig::default(),
                );
                let mut cache_entries: Vec<(
                    (starter_ext_spi::ExtensionId, String),
                    starter_cache::CacheSpec,
                )> = Vec::new();
                for record in bundle.registry.iter_validated() {
                    let Some(ext_id) = record.id.clone() else {
                        continue;
                    };
                    let kinds_dir = record.bundle_dir.join("kinds");
                    if !kinds_dir.is_dir() {
                        continue;
                    }
                    match KindCacheRegistry::load_from_dir(&ext_id, &kinds_dir) {
                        Ok((reg, errors)) => {
                            for err in &errors {
                                warn!(
                                    target: "rubix.boot.extensions",
                                    extension = %ext_id.as_str(),
                                    path = %err.path.display(),
                                    error = %err.message,
                                    "cache sidecar failed to load — kind will be uncached",
                                );
                            }
                            // KindCacheRegistry doesn't expose its
                            // entries; rebuild the (key, spec) pairs
                            // by re-walking the dir is wasteful — but
                            // load_from_dir already returns a Registry
                            // typed for one extension. We merge by
                            // re-loading sidecars one-by-one against
                            // the merged vec.
                            for ent in std::fs::read_dir(&kinds_dir)
                                .into_iter()
                                .flatten()
                                .flatten()
                            {
                                let p = ent.path();
                                let name = match p.file_name().and_then(|n| n.to_str()) {
                                    Some(n) => n,
                                    None => continue,
                                };
                                let Some(stem) = name.strip_suffix(".cache.yaml") else {
                                    continue;
                                };
                                if let Some(spec) = reg.get(&ext_id, stem) {
                                    cache_entries.push((
                                        (ext_id.clone(), stem.to_string()),
                                        spec.clone(),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                target: "rubix.boot.extensions",
                                extension = %ext_id.as_str(),
                                error = %e,
                                "failed to scan kinds/ for cache sidecars",
                            );
                        }
                    }
                }
                if !cache_entries.is_empty() {
                    info!(
                        target: "rubix.boot.extensions",
                        kinds = cache_entries.len(),
                        "opt-in cache: registered kind specs",
                    );
                }
                let kind_cache =
                    KindCacheRegistry::from_entries(cache_entries.iter().cloned());
                let dispatcher_cache = DispatcherCache::new(cache_layer.clone(), kind_cache);
                // Thread the layer into the admin state so
                // `GET /api/v1/admin/cache/specs` can render per-spec
                // hit/miss numbers — the canary's "is usage_bucketed
                // paying off?" panel reads this.
                admin_state = admin_state.with_cache_layer(cache_layer.clone());

                // Builtin dispatcher: in-process extensions go through the
                // capability factory above.
                let builtin = Arc::new(
                    BuiltinRestDispatcher::new(table, bundle.registry.clone())
                        .with_capability_factory(factory)
                        .with_cache(dispatcher_cache.clone()),
                );
                // Process dispatcher: child-process extensions are dispatched
                // through their supervisor handle, populated by
                // `boot::extensions::build_extension_admin`. Empty handle
                // map (no enabled process extensions) is fine — the
                // composite simply never picks this branch.
                let process = Arc::new(
                    ProcessRestDispatcher::new(
                        bundle.process_handles.clone(),
                        EXTENSION_REST_REQUEST_TIMEOUT,
                    )
                    .with_cache(dispatcher_cache),
                );
                let dispatcher: Arc<dyn RestDispatcher> = Arc::new(CompositeRestDispatcher::new(
                    bundle.registry.clone(),
                    builtin,
                    process,
                ));
                match rest_router::<()>(
                    bundle.registry.clone(),
                    dispatcher,
                    RestRouterOptions::default(),
                ) {
                    Ok(adapter) => {
                        let gated = with_principal(
                            with_caller_identity(adapter),
                            auth.authenticator.clone(),
                        );
                        app = app.merge_external(Router::new().nest("/api/v1", gated));
                    }
                    Err(e) => {
                        warn!(
                            target: "rubix.boot.extensions",
                            error = %e,
                            "extension REST adapter failed to build; per-extension \
                             routes will not be served",
                        );
                    }
                }
            } else {
                info!(
                    target: "rubix.boot.extensions",
                    "warehouse_url unset — skipping extension REST adapter wiring",
                );
            }
        }

        // Goal 1, Phase A.1 — seed bundled SDUI dashboard pages
        // (e.g. `dashboard.disk-overview`) into
        // `dashboards_definitions` before the page provider goes
        // live. Idempotent: a row already present for
        // `(BUNDLED_TENANT, page_id)` is skipped. The throwaway
        // `StaticRegistry` only carries the in-process
        // `try_register` call inside the seed — the real authz
        // engine above already declared the same kind, so this
        // registry is dropped after the seed returns.
        let seed_registry = starter_authz::StaticRegistry::new();
        let inserted = boot::dashboards_seed::seed(Some(&pool), &seed_registry)
            .await
            .map_err(|e| anyhow::anyhow!("dashboards_seed::seed: {e}"))?;
        tracing::info!(inserted, "dashboards_definitions seed complete",);

        // Phase B.2 — mount the SDUI router under `/api/v1/ui`.
        // `starter_sdui_routes::sdui_router` already roots its
        // routes at the full path, so we `merge` (not `nest`). The
        // four trait impls are composed inside
        // [`boot::build_sdui_router`] — verb-per-file.
        let sdui_router: Router =
            boot::build_sdui_router(&cfg, pool.clone(), warehouse_client.clone(), &tools);
        app = app.merge_external(sdui_router);

        // Live sidebar SSE — `GET /api/v1/dashboards/events`.
        // Tenant-scoped tail of `starter_changes` projected into
        // `created`/`updated`/`deleted` frames so the rubix frontend
        // sidebar updates the moment the chat surface (or any
        // operator) calls a `rubix.dashboard.*` write verb. See
        // `rubix/docs/scope/dashboards/09-live-sidebar-sse.md` and
        // [`routes::dashboard_events`]. Wrapped in `with_principal`
        // so anonymous subscribers see 401 before any stream opens.
        {
            use rubix_store_postgres::PgDashboardStore;
            use sqlx::postgres::PgPoolOptions;
            use starter_changelog_postgres::PgListenTail;
            use starter_server::auth::with_principal;
            use starter_store_postgres::pool::Pool;

            // Dedicated tiny pool for the SSE listener.
            // `PgListenTail` now uses ONE shared `PgListener` per
            // tail instance and fans out to N subscribers via a
            // `tokio::broadcast`, so the connection cost is fixed
            // regardless of how many browser tabs / hooks are
            // subscribed. Cap is 2 because the listener pins one
            // connection for its lifetime and the initial-cursor
            // snapshot + any `Lagged` gap-fills need a second
            // checkout (without it, a `subscribe()` that hits an
            // already-running listener would block on its own
            // cursor query forever). Isolated from the main 16-conn
            // pool so a sidebar storm cannot starve other auth-gated
            // routes (mirrors `boot::flow_notify`).
            //
            // History: pre-fan-out this pool was sized "1 per
            // subscriber" with all the documented hazards (silent
            // pinning, freeze cascade through the auth pool). See
            // `rubix/crates/rubix-watchdog/README.md` for the full
            // story; commit 7211aa9 raised the cap to 16 as a
            // band-aid before the fan-out refactor landed.
            let listen_inner = PgPoolOptions::new()
                .max_connections(2)
                .connect(dsn)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("connect for dashboard_events listen pool: {e}")
                })?;
            let listen_pool = Pool::from_sqlx(listen_inner);
            let _t_dash_listen =
                boot::pool_telemetry::spawn(listen_pool.sqlx().clone(), "rubix-dash-listen");

            let tail = Arc::new(PgListenTail::new(listen_pool));
            let store = Arc::new(PgDashboardStore::new(pool.clone()));
            let de_registrar = routes::dashboard_events::registrar(
                routes::dashboard_events::DashboardEventsState { tail, store },
            )
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(de_registrar);
        }

        // Chat streaming SSE — `POST /api/v1/chat/stream`. Direct
        // bridge from the Claude CLI wrapper's per-chunk Event
        // stream to the chat UI; bypasses the flow engine
        // entirely. See `rubix/docs/sessions/2026-05-25-dashboards-
        // sidebar-sse-and-chat-gaps.md` §"Part 3" and
        // [`routes::chat_stream`]. AuthN-gated via `with_principal`
        // so an anonymous POST sees 401 before any runner spawns.
        // The MCP wiring (so the model can dispatch host tools
        // mid-turn) reads `RUBIX_SERVICE_MCP_URL` +
        // `RUBIX_SERVICE_MCP_TOKEN`; unset means narration only.
        {
            use starter_server::auth::with_principal;
            let chat_runner = boot::ai::build_runner(&cfg)
                .map_err(|e| anyhow::anyhow!("boot::ai::build_runner (chat): {e}"))?;
            let chat_registrar = routes::chat_stream::registrar(
                routes::chat_stream::ChatStreamState::from_env(chat_runner),
            )
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(chat_registrar);
        }

        // Stage 07 — `POST /api/v1/flows/{id}/run`. Synchronous
        // human-driven flow invocation. Same `Arc<ToolRegistry>` the
        // MCP surface dispatches against, so a flow fired here goes
        // through the identical `FlowAsTool::invoke` path the AI uses
        // via `mcp.tools/call`. AuthN-gated; no audit / changelog
        // (those layers are tools-router specific).
        {
            use starter_server::auth::with_principal;
            let flow_run_registrar = routes::flow_run::registrar(routes::flow_run::FlowRunState {
                tools: mcp.tools.clone(),
            })
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(flow_run_registrar);
        }

        // Layer order matters. The changelog middleware reads
        // `Principal` from request extensions, so it must run
        // *inside* `with_principal`. We therefore audit the
        // tools router first, then wrap the audited router in
        // the auth + authz gate.
        //
        // The same recorder is shared with the admin invoke +
        // streaming-invoke routers below — every successful
        // dispatch through any rubix-agent-owned tool surface
        // lands one `Change` row, attributed to the authenticated
        // principal and scoped to the path's `tool_id` segment.
        let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool));
        let tool_audit_prefixes = vec![
            "/api/v1/tools/".to_owned(),
            "/api/v1/admin/registry/tools/".to_owned(),
        ];
        let tools_recorder = recorder.clone();
        let tools_audit_prefixes = tool_audit_prefixes.clone();
        let tools_gated = tools_registrar.map_router(|r| {
            let audited = middleware::changelog_layer(
                r,
                middleware::ChangelogState {
                    recorder: tools_recorder,
                    tool_path_prefixes: tools_audit_prefixes,
                },
            );
            middleware::gate_tools(audited, auth.authenticator.clone(), engine)
        });
        app = app
            .merge_external(Router::new().nest("/api/v1", auth_routes))
            .merge(tools_gated);

        // Admin introspection surface (`/api/v1/admin/*`).
        //
        // Two routers, gated independently:
        //
        // - **Read** (`admin_router`) — every `GET /admin/*`
        //   endpoint. Gated by `with_role(Role::Admin)` because the
        //   surface enumerates every tool, template, table and
        //   rule the agent advertises.
        // - **Invoke** (`admin_invoke_router`) — `POST
        //   /api/v1/admin/registry/tools/{id}/invoke`. Same
        //   `Role::Admin` gate plus an additional
        //   `with_scope("admin:invoke")` so an `admin:read`-only
        //   principal can browse the catalog but cannot fire
        //   tools.
        //
        // See docs/design/admin/README.md "Security model".
        {
            use starter_server::auth::{with_principal, with_role, with_scope};
            use starter_spi::auth::{Role, Scope};
            let read = routes::admin::admin_registrar(admin_state.clone())
                .map_router(|r| with_role(r, Role::Admin));
            // Both invoke surfaces sit behind the same audit
            // layer as the public dispatcher (see `tool_audit_*`
            // above). Each request lands one `Change` row keyed
            // on `tool.invoke` + the path's tool id, attributed
            // to the admin's authenticated principal.
            let admin_invoke_recorder = recorder.clone();
            let admin_invoke_prefixes = tool_audit_prefixes.clone();
            let invoke = routes::admin::admin_invoke_registrar(admin_state.clone())
                .map_router(move |r| {
                    let audited = middleware::changelog_layer(
                        r,
                        middleware::ChangelogState {
                            recorder: admin_invoke_recorder,
                            tool_path_prefixes: admin_invoke_prefixes,
                        },
                    );
                    with_scope(with_role(audited, Role::Admin), Scope::new("admin:invoke"))
                });
            let admin_stream_recorder = recorder.clone();
            let admin_stream_prefixes = tool_audit_prefixes.clone();
            let invoke_stream = routes::admin::admin_invoke_stream_registrar(admin_state.clone())
                .map_router(move |r| {
                    let audited = middleware::changelog_layer(
                        r,
                        middleware::ChangelogState {
                            recorder: admin_stream_recorder,
                            tool_path_prefixes: admin_stream_prefixes,
                        },
                    );
                    with_scope(with_role(audited, Role::Admin), Scope::new("admin:invoke"))
                });
            let gated_admin = read
                .merge(invoke)
                .merge(invoke_stream)
                .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(gated_admin);
        }
    } else {
        warn!(
            target: "rubix.boot",
            "RUBIX_DATABASE_URL unset — mounting tools router without auth/authz/audit gates",
        );
        let flow_run_registrar = routes::flow_run::registrar(routes::flow_run::FlowRunState {
            tools: mcp.tools.clone(),
        });
        app = app.merge(tools_registrar).merge(flow_run_registrar);

        // Even without DB auth, expose the admin introspection
        // surface so a laptop dev session can browse the registry.
        // Mounts ungated, mirroring the ungated `/api/v1/tools/*`
        // path above; production always sets `RUBIX_DATABASE_URL`
        // and lands in the gated branch. Invoke is exposed too —
        // the dev loop needs to fire tools through the same code
        // path the gated surface uses.
        app = app
            .merge(routes::admin::admin_registrar(admin_state.clone()))
            .merge(routes::admin::admin_invoke_registrar(admin_state.clone()))
            .merge(routes::admin::admin_invoke_stream_registrar(admin_state.clone()));
    }

    // Project the final route catalog into the admin OpenAPI doc.
    // The projection captures every rubix-agent-owned endpoint —
    // upstream routers (`starter-auth-users`, `starter-ext-server`,
    // `starter-warehouse-explorer`, `starter-sdui-routes`, MCP)
    // are deliberately excluded; they own their own OpenAPI
    // surface. The doc is snapshot here, *before* mounting the
    // `/api/v1/admin/openapi.json` route itself, so the projection
    // does not include a self-referential entry.
    let admin_openapi_doc = std::sync::Arc::new(routes::catalog_to_openapi(
        app.catalog(),
        routes::OpenApiInfo {
            title: "rubix-agent (admin projection)".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: Some(
                "Strict projection of routes mounted through RouteRegistrar — \
                 the live router and this doc cannot drift."
                    .to_owned(),
            ),
        },
    ));
    app = app.merge(routes::admin::admin_openapi_registrar(admin_openapi_doc));

    // Apply a permissive CORS layer so browser clients (the Flutter
    // web build served from a different origin during `flutter run
    // -d chrome`, plus any future SPA hosted off-host) can reach the
    // REST surface. `very_permissive` mirrors the default that
    // `starter-server::ServerBuilder` applies; tighten via a config
    // knob once we have a non-dev deployment story.
    let app = app
        .into_router()
        .layer(tower_http::cors::CorsLayer::very_permissive());

    health::serve(&cfg.bind, app).await
}
