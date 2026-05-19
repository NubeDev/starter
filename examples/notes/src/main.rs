//! `notes` — demo binary proving every starter surface is extensible
//! from a consumer crate. Subcommands:
//!
//! - `migrate` — apply notes + starter-auth-token migrations.
//! - `serve` — run REST/MCP (axum) and gRPC (tonic) on different ports.
//! - `claim` — issue a fresh pending claim and print the new bearer.
//! - `add` / `list` — consumer-owned CLI commands that go through HTTP.
//! - `health` / `openapi` — shipped by `register_starter_defaults`.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use starter_auth_token::{regenerate_claim_pending, store::SqliteClaimStore};
use starter_cli::registry::CommandRegistry;
use starter_notes::cli::{NoteAdd, NoteList};
use starter_notes::{grpc as notes_grpc, migrations, server as notes_server};
use starter_observability::{metrics::StandardMetrics, tracing::Format};
use starter_store_sqlite::{migrate, pool, Pool};

const DEFAULT_DATABASE_URL: &str = "sqlite:./notes.db?mode=rwc";
const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8080";
const DEFAULT_GRPC_BIND: &str = "127.0.0.1:50051";

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = starter_observability::tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    // Register starter defaults (`health`, `openapi`) AND the
    // consumer's own `add` / `list`. The registry doesn't care about
    // the source — `Command` trait is the boundary.
    let registry = CommandRegistry::new()
        .register_starter_defaults()
        .register(NoteAdd)
        .register(NoteList);

    let app = Command::new("notes")
        .about("Notes demo — every surface extends starter-* libraries with zero starter edits.")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("migrate")
                .about("Apply migrations (starter_auth_token + notes)")
                .arg(database_url_arg()),
        )
        .subcommand(
            Command::new("serve")
                .about("Run HTTP + gRPC servers")
                .arg(database_url_arg())
                .arg(Arg::new("http-bind").long("http-bind").default_value(DEFAULT_HTTP_BIND))
                .arg(Arg::new("grpc-bind").long("grpc-bind").default_value(DEFAULT_GRPC_BIND)),
        )
        .subcommand(
            Command::new("claim")
                .about("Regenerate the pending claim and print the new bearer")
                .arg(database_url_arg())
                .arg(Arg::new("yes").long("yes").short('y').action(ArgAction::SetTrue)),
        )
        .subcommands(registry.subcommands());

    let matches = app.get_matches();
    match matches.subcommand() {
        Some(("migrate", sub)) => run_migrate(sub).await,
        Some(("serve", sub)) => run_serve(sub).await,
        Some(("claim", sub)) => run_claim(sub).await,
        _ => registry.dispatch(&matches).await.map_err(|e| anyhow::anyhow!("{e:?}")),
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
    pool::connect(url).await.with_context(|| format!("connect to {url}"))
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

    let built = notes_server::build(pool, registry, metrics);

    let http_bind: SocketAddr = matches
        .get_one::<String>("http-bind")
        .map(String::as_str)
        .unwrap_or(DEFAULT_HTTP_BIND)
        .parse()
        .context("parse --http-bind")?;
    let grpc_bind: SocketAddr = matches
        .get_one::<String>("grpc-bind")
        .map(String::as_str)
        .unwrap_or(DEFAULT_GRPC_BIND)
        .parse()
        .context("parse --grpc-bind")?;

    let grpc = notes_grpc::NotesGrpc {
        store: built.store.clone(),
        authenticator: built.authenticator.clone(),
    };

    tracing::info!(%http_bind, %grpc_bind, "notes serving");

    // Run HTTP and gRPC concurrently. The first to error returns.
    tokio::select! {
        r = starter_server::builder::bind(built.router, http_bind) => r.context("http serve")?,
        r = tonic::transport::Server::builder()
            .add_service(grpc.into_server())
            .serve(grpc_bind) => r.context("grpc serve")?,
    }
    Ok(())
}

async fn run_claim(matches: &ArgMatches) -> Result<()> {
    if !matches.get_flag("yes") {
        eprintln!("refusing to regenerate without --yes — current owner token will be invalidated");
        std::process::exit(2);
    }
    let pool = open_pool(matches).await?;
    let store = SqliteClaimStore::new(pool);
    let pending = regenerate_claim_pending(&store).await.context("regenerate claim pending")?;
    println!("{}", pending.plaintext);
    Ok(())
}
