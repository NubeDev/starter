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

    let built = fa_server::build(pool, registry, metrics);

    let addr: SocketAddr = bind.parse().context("parse HTTP_BIND")?;
    tracing::info!(%addr, "flow-agent serving");
    starter_server::builder::bind(built.router, addr)
        .await
        .context("http serve")?;
    Ok(())
}
