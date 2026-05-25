//! `ch-explorer` — example binary that mounts the ClickHouse
//! explorer end-to-end: the `/api/warehouse/ch/*` read-only sub-
//! router from `starter-warehouse::explorer` and the prebuilt SPA
//! at `packages/starter-ui-ch-explorer/dist`. Designed to be the
//! single command an operator runs to confirm PR 3 of
//! `rubix/docs/scope/clickhouse-explorer.md` is wired end-to-end.
//!
//! Subcommands:
//!
//!   serve  — bind 127.0.0.1:3030, serve the UI + API.
//!   seed   — load a handful of demo tables + rows into the
//!            configured ClickHouse so a fresh dev box isn't an
//!            empty database.
//!
//! Environment:
//!
//!   CH_EXPLORER_URL       HTTP endpoint of ClickHouse (default
//!                         http://127.0.0.1:8123)
//!   CH_EXPLORER_DATABASE  Database name (default `default`)
//!   CH_EXPLORER_USER      Username (default `default`)
//!   CH_EXPLORER_PASSWORD  Password (default empty)
//!   CH_EXPLORER_BIND      Bind address (default 127.0.0.1:3030)
//!   CH_EXPLORER_DIST      Path to the built SPA dist directory
//!                         (default packages/starter-ui-ch-explorer/dist)
//!
//! Auth: the example mounts a dev-only `AllowAll` policy engine
//! and injects an anonymous `Role::Admin` principal via
//! `starter_server::auth::with_anonymous_principal`, so every
//! request satisfies the `("warehouse", "read")` gate without a
//! login. Do not copy this wiring into a production binary — see
//! the "When NOT to use this" note in the anonymous-layer module.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Extension;
use clap::{Arg, ArgMatches, Command};
use starter_authz::testing::AllowAll;
use starter_authz::with_permission;
use starter_observability::{metrics::StandardMetrics, tracing::Format};
use starter_server::auth::{local_operator, with_anonymous_principal};
use starter_server::ServerBuilder;
use starter_spi::authz::PolicyEngine;
use starter_store_clickhouse::{ChClient, ChConfig};

const DEFAULT_CH_URL: &str = "http://127.0.0.1:8123";
const DEFAULT_CH_DATABASE: &str = "default";
const DEFAULT_CH_USER: &str = "default";
const DEFAULT_BIND: &str = "127.0.0.1:3030";
const DEFAULT_DIST: &str = "packages/starter-ui-ch-explorer/dist";

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = starter_observability::tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    let app = Command::new("ch-explorer")
        .about("ClickHouse explorer example: serve the UI + API, or seed sample data.")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("serve")
                .about("Bind the HTTP server and serve the explorer SPA + API.")
                .arg(
                    Arg::new("bind")
                        .long("bind")
                        .default_value(DEFAULT_BIND)
                        .help("[host:port] (also CH_EXPLORER_BIND)"),
                )
                .arg(
                    Arg::new("dist")
                        .long("dist")
                        .help("Path to the built SPA dist (also CH_EXPLORER_DIST)"),
                ),
        )
        .subcommand(
            Command::new("seed")
                .about("Load a small fixture of tables + rows into ClickHouse."),
        );

    let matches = app.get_matches();
    match matches.subcommand() {
        Some(("serve", sub)) => run_serve(sub).await,
        Some(("seed", _)) => run_seed().await,
        _ => unreachable!(),
    }
}

fn ch_from_env() -> ChClient {
    let url = std::env::var("CH_EXPLORER_URL").unwrap_or_else(|_| DEFAULT_CH_URL.into());
    let database =
        std::env::var("CH_EXPLORER_DATABASE").unwrap_or_else(|_| DEFAULT_CH_DATABASE.into());
    let user = std::env::var("CH_EXPLORER_USER").unwrap_or_else(|_| DEFAULT_CH_USER.into());
    let password = std::env::var("CH_EXPLORER_PASSWORD").unwrap_or_default();
    let cfg = ChConfig {
        url,
        database,
        user,
        password,
        async_insert: true,
    };
    tracing::info!(url = %cfg.url, database = %cfg.database, "connecting to ClickHouse");
    ChClient::connect(cfg)
}

async fn run_serve(matches: &ArgMatches) -> Result<()> {
    let ch = ch_from_env();

    let dist: PathBuf = matches
        .get_one::<String>("dist")
        .cloned()
        .or_else(|| std::env::var("CH_EXPLORER_DIST").ok())
        .unwrap_or_else(|| DEFAULT_DIST.into())
        .into();
    if !dist.join("index.html").exists() {
        anyhow::bail!(
            "SPA dist not found at {} (run `pnpm -F @nube/starter-ui-ch-explorer build` first, \
             or override with --dist / CH_EXPLORER_DIST)",
            dist.display(),
        );
    }

    let bind: SocketAddr = matches
        .get_one::<String>("bind")
        .cloned()
        .or_else(|| std::env::var("CH_EXPLORER_BIND").ok())
        .unwrap_or_else(|| DEFAULT_BIND.into())
        .parse()
        .context("parse bind")?;

    // Dev-only authz: AllowAll engine + anonymous Admin principal
    // so every `/api/warehouse/ch/*` request satisfies the
    // ("warehouse", "read") gate without a login flow.
    let engine: Arc<dyn PolicyEngine> = Arc::new(AllowAll);
    let principal = local_operator("local:ch-explorer");

    let explorer_routes =
        with_permission(starter_warehouse::explorer::routes(ch), "warehouse", "read");
    let explorer_routes = with_anonymous_principal(explorer_routes, principal);
    let explorer_routes = explorer_routes.layer(Extension(engine));

    let registry = Arc::new(prometheus::Registry::new());
    let metrics = Arc::new(
        StandardMetrics::register(&registry).context("register prometheus metrics")?,
    );

    let router = ServerBuilder::<()>::new(())
        .merge_router(explorer_routes)
        .with_static_assets("/warehouse/explorer", dist)
        .with_metrics(registry, metrics)
        .build();

    tracing::info!(
        %bind,
        "ch-explorer: SPA at http://{bind}/warehouse/explorer, API at /api/warehouse/ch/*"
    );
    starter_server::builder::bind(router, bind)
        .await
        .context("serve")?;
    Ok(())
}

async fn run_seed() -> Result<()> {
    let ch = ch_from_env();
    let conn = ch.inner();

    tracing::info!("seeding demo tables…");

    // Drop-and-recreate is idempotent and means re-running the seed
    // doesn't accumulate stale partitions.
    for ddl in seed_ddl() {
        conn.query(ddl)
            .execute()
            .await
            .with_context(|| format!("seed DDL failed: {ddl}"))?;
    }
    for ddl in seed_data() {
        conn.query(ddl)
            .execute()
            .await
            .with_context(|| format!("seed INSERT failed: {ddl}"))?;
    }

    tracing::info!("seed complete");
    Ok(())
}

fn seed_ddl() -> &'static [&'static str] {
    &[
        "DROP TABLE IF EXISTS demo_buildings",
        "DROP TABLE IF EXISTS demo_meters",
        "DROP TABLE IF EXISTS demo_samples",
        "CREATE TABLE demo_buildings (\
            id UInt32, \
            name String, \
            region LowCardinality(String)\
         ) ENGINE = MergeTree ORDER BY id",
        "CREATE TABLE demo_meters (\
            id UInt32, \
            building_id UInt32, \
            kind LowCardinality(String), \
            unit LowCardinality(String)\
         ) ENGINE = MergeTree ORDER BY id",
        "CREATE TABLE demo_samples (\
            ts DateTime64(3, 'UTC'), \
            meter_id UInt32, \
            value Float64\
         ) ENGINE = MergeTree PARTITION BY toYYYYMM(ts) ORDER BY (meter_id, ts)",
    ]
}

fn seed_data() -> &'static [&'static str] {
    &[
        "INSERT INTO demo_buildings (id, name, region) VALUES \
         (1, 'HQ', 'AU-NSW'), (2, 'Warehouse', 'AU-VIC'), (3, 'Lab', 'AU-QLD')",
        "INSERT INTO demo_meters (id, building_id, kind, unit) VALUES \
         (10, 1, 'kwh', 'kWh'), (11, 1, 'water', 'L'), \
         (20, 2, 'kwh', 'kWh'), (30, 3, 'temp', 'degC')",
        // 1 row per (meter, hour) for a single day so /tables shows
        // non-zero counts but the page weighs ~nothing.
        "INSERT INTO demo_samples (ts, meter_id, value) \
         SELECT \
            toDateTime64('2026-05-25 00:00:00', 3, 'UTC') + INTERVAL h HOUR, \
            m, \
            round(rand() % 1000 / 10.0, 2) \
         FROM (SELECT arrayJoin(range(24)) AS h) AS hh \
         CROSS JOIN (SELECT arrayJoin([10, 11, 20, 30]) AS m) AS mm",
    ]
}
