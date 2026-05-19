//! `starter-gh-report` — skeleton for a GitHub-reporting CLI built on
//! the starter contract.
//!
//! This is a **shape demonstration**, not a working integration: the
//! `report` subcommand prints a stubbed JSON body so the file count
//! stays small while the consumer-domain layout is clear. Replace
//! `report::generate` with a real `octocrab` call when the consumer
//! contract for the reporting tool is finalised.
//!
//! Demonstrates the **canonical layout** for a consumer-domain CLI on
//! top of `starter-cli`:
//!
//! - Register starter defaults (`health`, `openapi`) for any future
//!   server-mode subcommands.
//! - Add the domain subcommand (`report`) as a local `Command` impl.
//! - Use `starter-observability` for tracing init so logs match the
//!   rest of the starter ecosystem.

mod report;

use anyhow::Result;
use clap::Command;
use starter_cli::registry::CommandRegistry;
use starter_observability::tracing::{init, Format};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _tracing = init(&filter, Format::Pretty).map_err(|e| anyhow::anyhow!("tracing: {e}"))?;

    let registry = CommandRegistry::new()
        .register_starter_defaults()
        .register(report::Report);

    let app = Command::new("starter-gh-report")
        .about("GitHub reporting CLI — skeleton built on the starter contract.")
        .arg_required_else_help(true)
        .subcommands(registry.subcommands());

    let matches = app.get_matches();
    registry
        .dispatch(&matches)
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}
