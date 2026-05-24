//! `rubix-admin mcp` — stdio JSON-RPC MCP transport.
//!
//! A single-verb subcommand: launching the binary with no flags
//! enters the stdio loop. The implementation lives in
//! [`serve`] (verb-per-file). The subcommand exists so Claude
//! Desktop (and any other MCP-capable host) can spawn rubix as a
//! child process and talk to it over framed JSON-RPC on stdio.
//!
//! See [docs/design/agent/](../../../../../docs/design/agent/README.md)
//! and [docs/design/i18n-prefs/](../../../../../docs/design/i18n-prefs/README.md)
//! for the stdio locale-cascade contract this verb satisfies.

pub mod serve;

use clap::Args as ClapArgs;

/// Flags for `rubix-admin mcp`. The verb takes no positional
/// arguments; `RUBIX_PRINCIPAL_EMAIL`, `RUBIX_CONFIG`, and `LANG`
/// are read from the process environment.
#[derive(Debug, ClapArgs)]
pub struct Args {}

pub async fn run(args: Args) -> anyhow::Result<()> {
    serve::run(args).await
}
