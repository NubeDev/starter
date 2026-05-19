//! `starter-minimal` — headless single-owner appliance.
//!
//! Subcommands:
//! - `migrate`     apply every namespaced migration source against
//!   `--database-url` (default `sqlite:./minimal.db`).
//! - `serve`       run the axum app on `--bind` (default `127.0.0.1:8080`).
//! - `claim-reset` factory-reset the auth-token claim flow and print
//!   the fresh pending claim token to stdout.
//! - `health`      hit `GET /health` on a running instance.
//! - `openapi`     fetch `GET /openapi.json` on a running instance.
//!
//! Configuration: env-var first (`DATABASE_URL`, `STARTER_BIND_ADDR`,
//! `RUST_LOG`), CLI flags override. No config file — this is the
//! 80-line example.

mod migrations;
mod server;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use starter_auth_token::{regenerate_claim_pending, store::SqliteClaimStore};
use starter_cli::registry::CommandRegistry;
use starter_observability::{metrics::StandardMetrics, tracing::Format};
use starter_store_sqlite::{migrate, pool, Pool};

const DEFAULT_DATABASE_URL: &str = "sqlite:./minimal.db?mode=rwc";
const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = starter_observability::tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    let registry = CommandRegistry::new().register_starter_defaults();
    let app = Command::new("starter-minimal")
        .about("Headless single-owner appliance — see --help on each subcommand.")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("migrate")
                .about("Apply migrations (starter_auth_token + app)")
                .arg(database_url_arg()),
        )
        .subcommand(
            Command::new("serve")
                .about("Run the server")
                .arg(database_url_arg())
                .arg(
                    Arg::new("bind")
                        .long("bind")
                        .env("STARTER_BIND_ADDR")
                        .default_value(DEFAULT_BIND),
                ),
        )
        .subcommand(
            Command::new("claim-reset")
                .about("Wipe the claimed owner-token + issue a fresh pending claim")
                .arg(database_url_arg())
                .arg(
                    Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(ArgAction::SetTrue)
                        .help("Skip the interactive confirmation"),
                ),
        )
        .subcommands(registry.subcommands());

    let matches = app.get_matches();
    match matches.subcommand() {
        Some(("migrate", sub)) => run_migrate(sub).await,
        Some(("serve", sub)) => run_serve(sub).await,
        Some(("claim-reset", sub)) => run_claim_reset(sub).await,
        _ => registry
            .dispatch(&matches)
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}")),
    }
}

fn database_url_arg() -> Arg {
    Arg::new("database-url")
        .long("database-url")
        .env("DATABASE_URL")
        .default_value(DEFAULT_DATABASE_URL)
}

async fn open_pool(matches: &ArgMatches) -> Result<Pool> {
    let url = matches
        .get_one::<String>("database-url")
        .map(String::as_str)
        .unwrap_or(DEFAULT_DATABASE_URL);
    pool::connect(url)
        .await
        .with_context(|| format!("connect to {url}"))
}

async fn run_migrate(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;
    let mut chain = migrate(&pool);
    for source in migrations::sources() {
        chain = chain.with_source(source);
    }
    chain.run().await.context("apply migrations")?;
    println!("migrations applied");
    Ok(())
}

async fn run_serve(matches: &ArgMatches) -> Result<()> {
    let pool = open_pool(matches).await?;

    let registry = std::sync::Arc::new(prometheus::Registry::new());
    let metrics = std::sync::Arc::new(
        StandardMetrics::register(&registry).context("register prometheus metrics")?,
    );

    let router = server::build(pool, registry, metrics);
    let bind: SocketAddr = matches
        .get_one::<String>("bind")
        .map(String::as_str)
        .unwrap_or(DEFAULT_BIND)
        .parse()
        .context("parse --bind")?;

    tracing::info!(%bind, "starter-minimal listening");
    starter_server::builder::bind(router, bind)
        .await
        .context("serve")?;
    Ok(())
}

async fn run_claim_reset(matches: &ArgMatches) -> Result<()> {
    if !matches.get_flag("yes") {
        eprintln!(
            "refusing to reset without --yes — this invalidates the current owner token immediately",
        );
        std::process::exit(2);
    }
    let pool = open_pool(matches).await?;
    let store = SqliteClaimStore::new(pool);
    let pending = regenerate_claim_pending(&store)
        .await
        .context("regenerate claim pending")?;
    println!("{}", pending.plaintext);
    Ok(())
}
