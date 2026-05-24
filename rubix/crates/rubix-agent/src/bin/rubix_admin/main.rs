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
mod mcp;
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
    /// Run the MCP stdio JSON-RPC transport so an MCP host
    /// (Claude Desktop, another agent) can talk to rubix as a
    /// child process. `RUBIX_PRINCIPAL_EMAIL` selects the actor;
    /// `RUBIX_CONFIG` points at the agent config; per-call
    /// `params._meta.acceptLanguage` selects the locale.
    Mcp(mcp::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Only the non-`mcp` verbs install the pretty tracing
    // subscriber. The stdio MCP transport reserves stdout for
    // framed JSON-RPC, and `starter_observability::tracing::init`
    // writes records to stdout by default; installing it here
    // would corrupt the wire. The `mcp` verb opts out and routes
    // operator-visible messages straight to stderr.
    let _guard = match cli.command {
        Command::Mcp(_) => None,
        _ => {
            let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
            Some(
                starter_observability::tracing::init(
                    &filter,
                    starter_observability::tracing::Format::Pretty,
                )
                .map_err(|e| anyhow::anyhow!("init tracing: {e}"))?,
            )
        }
    };

    match cli.command {
        Command::BootstrapUser(args) => bootstrap_user::run(args).await,
        Command::System(cmd) => system::run(cmd).await,
        Command::Mcp(args) => mcp::run(args).await,
    }
}
