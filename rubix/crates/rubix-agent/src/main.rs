//! Rubix backend binary.
//!
//! Wiring composition site. Boots `starter-observability`, loads the
//! layered [`boot::AgentConfig`], applies the rubix-owned Postgres +
//! ClickHouse migrations, builds the tool registry (threading a live
//! [`starter_store_clickhouse::ChClient`] through the disk tool), and
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
use starter_store_clickhouse::ChClient;
use starter_store_postgres::pool::connect as pg_connect;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

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
    let _undo_sweep =
        boot::spawn_undo_sweep(cfg.database_url.as_deref(), cfg.undo.clone()).await?;
    let ch_migrations = boot::apply_ch_migrations(
        cfg.clickhouse_url.as_deref(),
        cfg.database_url.as_deref(),
    )
    .await?;

    // The agent boots without a warehouse when `clickhouse_url` is
    // unset OR when the CH migration step skipped (no RUBIX_CH_URL,
    // no parseable DSN, etc.). Wiring a `ChClient` into the disk
    // tool when the `system_disk_history` table was never created
    // would 500 on every invocation — see
    // docs/design/warehouse/README.md for the gate contract.
    //
    // Build the `ChClient` BEFORE the MCP surface so both the REST
    // and the MCP/`ai-agent` tool snapshots persist history rows
    // identically.
    let ch_client: Option<Arc<ChClient>> = if ch_migrations.skipped {
        None
    } else {
        cfg.clickhouse_url
            .as_ref()
            .map(|url| Arc::new(ChClient::connect(boot::rubix_ch_config(url.clone()))))
    };
    // Reuse a single PG pool for the MCP surface (flows_definitions
    // seed + load) so we don't open a second connection pool just
    // to read flow YAMLs. `None` is the laptop path — MCP falls
    // back to the embedded bundle.
    let mcp_pool: Option<starter_store_postgres::pool::Pool> =
        match cfg.database_url.as_deref() {
            Some(dsn) => Some(
                pg_connect(dsn)
                    .await
                    .map_err(|e| anyhow::anyhow!("connect for mcp pool: {e}"))?,
            ),
            None => None,
        };

    // SCOPE OQ-4: build the extension admin BEFORE the MCP surface so
    // the surface can emit one MCP tool per
    // `contributes.tools[]` entry alongside the bundled `FlowAsTool`
    // entries. The ordering is load-bearing: starter-mcp's
    // `ToolRegistry` is wrapped in `Arc` once `build_mcp_surface`
    // returns, so any extension tool not registered here is silently
    // missing from `tools/list`. The PG pool acquired here is reused
    // below for the auth + changelog sandwich — keeping the
    // connection-pool count at one.
    let ext_bundle: Option<boot::ExtensionAdminBundle> =
        match (mcp_pool.as_ref(), cfg.extensions.enabled) {
            (Some(pool), true) => Some(boot::build_extension_admin(&cfg, pool.sqlx()).await?),
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
        ext_bundle
            .as_ref()
            .map(|b| boot::mcp::ExtensionMcpContext {
                registry: b.registry.clone(),
                process_handles: b.process_handles.clone(),
            });
    let mcp = boot::mcp::build_mcp_surface(
        ch_client.clone(),
        mcp_pool.clone(),
        ext_mcp_ctx.as_ref(),
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
                Some(handle)
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
    .await?;
    // Clone the optional `ChClient` for the SDUI query engine before
    // the registry consumes the original. Phase B.2: the SDUI
    // `QueryEngine` honours `ch:<table>` prefixes against the same
    // warehouse the disk tool persists history into.
    let ch_client_for_sdui = ch_client.clone();
    let tools = registry::build_tool_registry(ch_client, cfg.insights.disk_warn_threshold);

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
    let tools_router = routes::tools::router(tools_state);

    // Phase C.2 — always-on flow runtime. Constructs the shared
    // `FlowSubscriptionRegistry` consumed by the SSE
    // `/api/v1/flows/{flow_id}/events` route plus the `NodeStateStore`
    // seam threaded into every `NodeCtx::state` call site (today the
    // engine still owns its own store wiring; the runtime hands its
    // `Arc<dyn NodeStateStore>` out for upstream engine reuse as the
    // per-flow event pump lands in a follow-up stage).
    let flow_runtime =
        boot::build_flow_runtime(cfg.database_url.as_deref(), &cfg.flow_runtime).await?;
    let flow_events_router =
        routes::flow_events::router(routes::flow_events::FlowEventsState {
            subscriptions: flow_runtime.subscriptions.clone(),
        });
    // Surface the runtime via the same `_` leak pattern as the other
    // always-on boot pieces — the `Arc<dyn NodeStateStore>` rides on
    // it so a future stage can hand it to the engine without another
    // boot-time refactor.
    let _flow_runtime = flow_runtime;

    // ----------------------------------------------------------------
    // Compose. The auth + authz + changelog sandwich layers in only
    // when a database is configured; without a DSN the binary still
    // serves /healthz + /api/v1/mcp + an ungated /api/v1/tools/*
    // surface so a developer can drive the agent on a laptop. The
    // production smoke path always sets RUBIX_DATABASE_URL.
    // ----------------------------------------------------------------
    // The MCP router builds its own `/mcp` route; nest under
    // `/api/v1` so the production surface lands at
    // `POST /api/v1/mcp` per docs/design/mcp-ux/README.md.
    let mcp_routes = Router::new().nest("/api/v1", mcp.router);
    let openapi_doc = routes::openapi_doc::openapi_router(rubix_openapi_mod::rubix_openapi());
    let mut app: Router = health::healthz_router()
        .merge(mcp_routes)
        .merge(openapi_doc)
        // SSE flow-events route. CSRF-exempt (mirrors the
        // extensions-events route): `EventSource` cannot send a CSRF
        // token and `text/event-stream` GETs carry no body. AuthN
        // still gates the route under the standard `with_principal`
        // sandwich when a DSN is set; without a DSN the laptop dev
        // path leaves it open alongside the tools router.
        .merge(flow_events_router);

    if let Some(dsn) = cfg.database_url.as_deref() {
        let pool = pg_connect(dsn)
            .await
            .map_err(|e| anyhow::anyhow!("connect to RUBIX_DATABASE_URL: {e}"))?;
        let auth = boot::build_auth(pool.clone());
        let auth_routes = routes::auth::auth_router(auth.state);
        let engine = boot::authz::build_engine()?;

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
            let ext_router: Router =
                starter_ext_server::router_with_auth(bundle.admin, auth.authenticator.clone());
            app = app.merge(Router::new().nest("/api/v1", ext_router));
        }

        // Phase B.2 — mount the SDUI router under `/api/v1/ui`.
        // `starter_sdui_routes::sdui_router` already roots its
        // routes at the full path, so we `merge` (not `nest`). The
        // four trait impls are composed inside
        // [`boot::build_sdui_router`] — verb-per-file.
        let sdui_router: Router =
            boot::build_sdui_router(&cfg, pool.clone(), ch_client_for_sdui.clone(), &tools);
        app = app.merge(sdui_router);

        // Layer order matters. The changelog middleware reads
        // `Principal` from request extensions, so it must run
        // *inside* `with_principal`. We therefore audit the
        // tools router first, then wrap the audited router in
        // the auth + authz gate.
        let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool));
        let audited = middleware::changelog_layer(
            tools_router,
            middleware::ChangelogState {
                recorder,
                tool_path_prefix: "/api/v1/tools/".to_owned(),
            },
        );
        let gated = middleware::gate_tools(
            audited,
            auth.authenticator.clone(),
            engine,
        );
        app = app
            .merge(Router::new().nest("/api/v1", auth_routes))
            .merge(gated);
    } else {
        warn!(
            target: "rubix.boot",
            "RUBIX_DATABASE_URL unset — mounting tools router without auth/authz/audit gates",
        );
        app = app.merge(tools_router);
    }

    health::serve(&cfg.bind, app).await
}
