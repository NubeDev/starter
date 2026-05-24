//! `rubix-admin system <verb>` — in-process operator probes.
//!
//! One verb per file. Each verb calls the same `probe()` the REST
//! handler dispatches in-process — no TCP round-trip to the agent.
//! See [docs/design/tools/](../../../../../docs/design/tools/README.md).

pub mod disk;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum SystemCommand {
    /// Probe disk usage on the local host and render the diagnostic
    /// in the operator's locale (`$LANG`).
    Disk(disk::Args),
}

pub async fn run(cmd: SystemCommand) -> anyhow::Result<()> {
    match cmd {
        SystemCommand::Disk(args) => disk::run(args).await,
    }
}
