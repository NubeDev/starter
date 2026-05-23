//! `flow-agent` — single entry: boot the HTTP server. No CLI
//! subcommands (per SCOPE F5). `cargo run -p flow-agent` runs migrate
//! + serve; everything else (auth, claim issuance) is intentionally absent.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use starter_observability::{metrics::StandardMetrics, tracing::Format};
use starter_store_postgres::{migrate, pool};

use flow_agent::{migrations, server as fa_server};

const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8090";

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = starter_observability::tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    let database_url = std::env::var("DATABASE_URL").with_context(|| {
        "DATABASE_URL is required (e.g. postgres://user:pass@localhost:5432/flow_agent)"
    })?;
    let bind = std::env::var("HTTP_BIND").unwrap_or_else(|_| DEFAULT_HTTP_BIND.to_owned());

    let pool = pool::connect(&database_url)
        .await
        .with_context(|| format!("connect to {database_url}"))?;

    let mut chain = migrate(&pool);
    for source in migrations::sources() {
        chain = chain.with_source(source);
    }
    chain.run().await.context("apply migrations")?;
    tracing::info!("migrations applied");

    let registry = std::sync::Arc::new(prometheus::Registry::new());
    let metrics = std::sync::Arc::new(
        StandardMetrics::register(&registry).context("register prometheus metrics")?,
    );

    // Optional `warehouse` capability (W1–W16). Enabled at compile
    // time via `--features warehouse`. When on, we apply the
    // `dimensions` migrations on the same Postgres pool and build a
    // `WarehouseRuntime` against `CLICKHOUSE_URL`. The warehouse
    // REST surface (`/api/marts`, `/api/sandboxes`,
    // `/api/warehouse/{status,gc,audit}`) is then merged onto the
    // assembled router. flow-agent is still Postgres-only for
    // OLTP — the warehouse seam reaches ClickHouse only via
    // `starter-store-clickhouse`, never directly.
    #[cfg(feature = "warehouse")]
    let built = {
        use std::sync::Arc;
        // 1. Dimensions migrations on PG.
        starter_store_postgres::migrate(&pool)
            .with_source(starter_store_postgres::dimensions::DIMENSIONS_MIGRATION_SOURCE)
            .run()
            .await
            .context("apply dimensions migrations")?;
        // 2. ClickHouse client config from env.
        let ch_url =
            std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://127.0.0.1:8123".into());
        let ch_db = std::env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "default".into());
        let ch_user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".into());
        let ch_pass = std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();
        let ch_cfg = starter_store_clickhouse::ChConfig {
            url: ch_url,
            database: ch_db.clone(),
            user: ch_user.clone(),
            password: ch_pass.clone(),
            async_insert: true,
        };
        // 3. Best-effort CH migrations. Skipped on connect failure;
        //    operator can re-run via /api/warehouse/status guard.
        let ch_client = starter_store_clickhouse::ChClient::connect(ch_cfg.clone());
        {
            let pg_host = std::env::var("WAREHOUSE_PG_HOST").unwrap_or_else(|_| "127.0.0.1".into());
            let pg_port: u16 = std::env::var("WAREHOUSE_PG_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432);
            let pg_user = std::env::var("WAREHOUSE_PG_USER").unwrap_or_else(|_| "postgres".into());
            let pg_pass = std::env::var("WAREHOUSE_PG_PASSWORD").unwrap_or_default();
            let pg_db = std::env::var("WAREHOUSE_PG_DB").unwrap_or_else(|_| "flow_agent".into());
            let pg_src = starter_store_clickhouse::PgSource {
                host: pg_host,
                port: pg_port,
                user: pg_user,
                password: pg_pass,
                db: pg_db,
            };
            let res = starter_store_clickhouse::MigrationRunner::new(&ch_client)
                .with_pg_source(pg_src)
                .run()
                .await;
            match res {
                Ok(()) => tracing::info!("warehouse: clickhouse migrations applied"),
                Err(e) => tracing::warn!(error = %e, "warehouse: clickhouse migrations skipped"),
            }
        }
        // 4. WarehouseRuntime.
        let rt = Arc::new(starter_warehouse::nodes::runtime::WarehouseRuntime::new(
            pool.clone(),
            ch_cfg,
            starter_warehouse::WarehouseConfig::default(),
        ));
        tracing::info!(
            "warehouse: runtime mounted under /api/marts, /api/sandboxes, /api/warehouse/*"
        );
        fa_server::build_with_warehouse(pool, registry, metrics, Some(rt))
    };
    #[cfg(not(feature = "warehouse"))]
    let built = fa_server::build(pool, registry, metrics);

    let addr: SocketAddr = bind.parse().context("parse HTTP_BIND")?;
    tracing::info!(%addr, "flow-agent serving");
    starter_server::builder::bind(built.router, addr)
        .await
        .context("http serve")?;
    Ok(())
}
