//! `rubix-admin` — operator CLI sibling to the agent binary.
//!
//! Subcommands are one-verb-per-file under this directory. The
//! binary itself is pure wiring: parse args, dispatch. See
//! [docs/design/auth/README.md](../../../../docs/design/auth/README.md)
//! for the bootstrap contract and
//! [docs/design/migrations/README.md](../../../../docs/design/migrations/README.md)
//! for the migration ordering this CLI relies on.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod bootstrap_user;
mod system;

#[derive(Debug, Parser)]
#[command(
    name = "rubix-admin",
    about = "Rubix operator CLI — first-run admin bootstrap and related verbs.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create (or confirm) the first admin user against the
    /// Postgres instance pointed at by `RUBIX_DSN`.
    BootstrapUser(bootstrap_user::Args),
    /// In-process system probes (disk, …) — share the same
    /// `probe()` the REST handler dispatches.
    #[command(subcommand)]
    System(system::SystemCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Same tracing shape the agent binary uses, so operators see
    // identical log formatting across `rubix-agent` and
    // `rubix-admin`.
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    let _guard = starter_observability::tracing::init(
        &filter,
        starter_observability::tracing::Format::Pretty,
    )
    .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?;

    let cli = Cli::parse();
    match cli.command {
        Command::BootstrapUser(args) => bootstrap_user::run(args).await,
        Command::System(cmd) => system::run(cmd).await,
    }
}
