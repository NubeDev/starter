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
use rubix_agent::{health, middleware, registry, routes};
use starter_changelog_postgres::PgChangeRecorder;
use starter_spi::changelog::ChangeRecorder;
use starter_store_clickhouse::{ChClient, ChConfig};
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

    let migrations = boot::apply_migrations().await?;
    let ch_migrations = boot::apply_ch_migrations(cfg.database_url.as_deref()).await?;
    let mcp = boot::mcp::build_mcp_surface().await?;

    // The agent boots without a warehouse when `clickhouse_url` is
    // unset; the disk tool then skips its history write.
    let ch_client: Option<Arc<ChClient>> = cfg.clickhouse_url.as_ref().map(|url| {
        Arc::new(ChClient::connect(ChConfig::local(url.clone())))
    });
    let tools = registry::build_tool_registry(ch_client);

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

    let tools_state = routes::tools::ToolsState::new(tools, bundle.clone());
    let tools_router = routes::tools::router(tools_state);

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
    let mut app: Router = health::healthz_router().merge(mcp_routes);

    if let Some(dsn) = cfg.database_url.as_deref() {
        let pool = pg_connect(dsn)
            .await
            .map_err(|e| anyhow::anyhow!("connect to RUBIX_DATABASE_URL: {e}"))?;
        let auth = boot::build_auth(pool.clone());
        let auth_routes = routes::auth::auth_router(auth.state);
        let engine = boot::authz::build_engine()?;

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
